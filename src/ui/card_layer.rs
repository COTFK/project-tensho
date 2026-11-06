use anyhow::{Context, ensure};

use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{
    CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
};
use crate::ocgcore::messages::{CoreMessage, DrawMessageData, MoveMessageData, SwapMessageData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CardId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CardAddress {
    pub controller: Player,
    pub location: CardLocation,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardPlacement {
    Zone(CardAddress),
    Overlay { host: CardId, sequence: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Card {
    pub id: CardId,
    pub code: u32,
    pub placement: CardPlacement,
    pub position: Option<CardPositionFlags>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CardLayer {
    cards: Vec<Card>,
    next_id: u64,
}

impl CardLayer {
    pub fn from_decks(main_deck: &[u32], extra_deck: &[u32]) -> anyhow::Result<Self> {
        let mut layer = Self::default();
        for (location, codes) in [
            (CardLocation::Deck, main_deck),
            (CardLocation::ExtraDeck, extra_deck),
        ] {
            for (sequence, &code) in codes.iter().enumerate() {
                layer.insert(
                    CardAddress {
                        controller: Player::Player,
                        location,
                        sequence: u32::try_from(sequence)?,
                    },
                    code,
                )?;
            }
        }
        Ok(layer)
    }

    pub fn insert(&mut self, address: CardAddress, code: u32) -> anyhow::Result<CardId> {
        ensure!(
            self.card_at(address).is_none(),
            "Card address is already occupied"
        );
        let id = CardId(self.next_id);
        self.next_id = self.next_id.checked_add(1).context("Card IDs exhausted")?;
        self.cards.push(Card {
            id,
            code,
            placement: CardPlacement::Zone(address),
            position: None,
        });
        Ok(id)
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn card_at(&self, address: CardAddress) -> Option<&Card> {
        self.cards
            .iter()
            .find(|card| card.placement == CardPlacement::Zone(address))
    }

    pub fn material_at(&self, host: CardId, sequence: u32) -> Option<&Card> {
        self.cards
            .iter()
            .find(|card| card.placement == CardPlacement::Overlay { host, sequence })
    }

    pub fn apply(&mut self, message: &CoreMessage) -> anyhow::Result<()> {
        let mut next = self.clone();
        match message {
            CoreMessage::Move(message) => next.apply_move(message)?,
            CoreMessage::Swap(message) => next.apply_swap(message)?,
            CoreMessage::Draw(message) => next.apply_draw(message)?,
            CoreMessage::PositionChange(message) => next.update_appearance(
                CardPlacement::Zone(CardAddress {
                    controller: message.controller,
                    location: message.location,
                    sequence: message.sequence,
                }),
                message.card_code,
                message.current_position,
            )?,
            CoreMessage::SetCard(message) => {
                next.apply_card_event(message.location, message.card_code)?
            }
            CoreMessage::Summoning(message)
            | CoreMessage::SpecialSummoning(message)
            | CoreMessage::FlipSummoning(message) => {
                next.apply_card_event(message.location, message.card_code)?
            }
            _ => anyhow::bail!("Unsupported card update"),
        }
        *self = next;
        Ok(())
    }

    fn apply_move(&mut self, message: &MoveMessageData) -> anyhow::Result<()> {
        let previous = self.placement_from(message.previous)?;
        ensure!(
            previous.is_some() || message.current.location != CardLocationKind::None,
            "Move has no card location"
        );
        let card = previous.map(|place| self.remove_card(place)).transpose()?;
        let current = self.placement_from(message.current)?;
        let position = match message.current.position {
            CardLocationPosition::Position(position) if current.is_some() => Some(position),
            _ => None,
        };
        self.place_moved_card(card, current, message.card_code, position)
    }

    fn apply_swap(&mut self, message: &SwapMessageData) -> anyhow::Result<()> {
        let (first, first_placement) =
            self.swap_card_source(message.first_location, message.first_code)?;
        let (second, second_placement) =
            self.swap_card_source(message.second_location, message.second_code)?;
        ensure!(first != second, "Swap locations are identical");
        self.cards[first].placement = second_placement;
        self.cards[second].placement = first_placement;
        Ok(())
    }

    fn swap_card_source(
        &mut self,
        info: CardLocationInfo,
        code: u32,
    ) -> anyhow::Result<(usize, CardPlacement)> {
        let placement = self.placement_from(info)?.context("Swap has no location")?;
        let CardPlacement::Zone(address) = placement else {
            anyhow::bail!("Swap targets an overlay material");
        };
        ensure!(!Self::is_pile(address.location), "Swap targets a pile");
        let CardLocationPosition::Position(position) = info.position else {
            anyhow::bail!("Swap card has no position");
        };
        let index = self
            .cards
            .iter()
            .position(|card| card.placement == placement)
            .context("No card at swap location")?;
        let card = &mut self.cards[index];
        Self::update_code(card, code)?;
        card.position = Some(position);
        Ok((index, placement))
    }

    fn apply_draw(&mut self, message: &DrawMessageData) -> anyhow::Result<()> {
        for drawn in &message.cards {
            let deck_len = self.pile_len(message.player, CardLocation::Deck)?;
            let hand_len = self.pile_len(message.player, CardLocation::Hand)?;
            let previous = CardAddress {
                controller: message.player,
                location: CardLocation::Deck,
                sequence: deck_len
                    .checked_sub(1)
                    .context("Cannot draw from an empty deck")?,
            };
            let current = CardAddress {
                controller: message.player,
                location: CardLocation::Hand,
                sequence: hand_len,
            };
            let card = self.remove_card(CardPlacement::Zone(previous))?;
            self.place_moved_card(
                Some(card),
                Some(CardPlacement::Zone(current)),
                drawn.code,
                Some(drawn.position),
            )?;
        }
        Ok(())
    }

    fn apply_card_event(&mut self, info: CardLocationInfo, code: u32) -> anyhow::Result<()> {
        let placement = self
            .placement_from(info)?
            .context("Card event has no location")?;
        ensure!(
            matches!(placement, CardPlacement::Zone(_)),
            "Card event targets an overlay material"
        );
        let CardLocationPosition::Position(position) = info.position else {
            anyhow::bail!("Card event has no position");
        };
        self.update_appearance(placement, code, position)
    }

    fn update_appearance(
        &mut self,
        placement: CardPlacement,
        code: u32,
        position: CardPositionFlags,
    ) -> anyhow::Result<()> {
        let card = self
            .cards
            .iter_mut()
            .find(|card| card.placement == placement)
            .context("No card at appearance update location")?;
        Self::update_code(card, code)?;
        card.position = Some(position);
        Ok(())
    }

    fn update_code(card: &mut Card, code: u32) -> anyhow::Result<()> {
        ensure!(
            card.code == 0 || code == 0 || card.code == code,
            "Message card code does not match the tracked card"
        );
        if code != 0 {
            card.code = code;
        }
        Ok(())
    }

    fn placement_from(&self, info: CardLocationInfo) -> anyhow::Result<Option<CardPlacement>> {
        match info.location {
            CardLocationKind::None => Ok(None),
            CardLocationKind::Known(location) => {
                let controller = info.controller.context("Card location has no controller")?;
                let CardLocationPosition::Position(_) = info.position else {
                    anyhow::bail!("Non-overlay card has an overlay sequence");
                };
                Ok(Some(CardPlacement::Zone(CardAddress {
                    controller,
                    location,
                    sequence: info.sequence,
                })))
            }
            CardLocationKind::Overlay { host } => {
                let CardLocationPosition::OverlaySequence(sequence) = info.position else {
                    anyhow::bail!("Overlay card has no material sequence");
                };
                let host = match host {
                    Some(location) => {
                        let address = CardAddress {
                            controller: info
                                .controller
                                .context("Overlay host has no controller")?,
                            location,
                            sequence: info.sequence,
                        };
                        self.card_at(address).context("Overlay host not found")?.id
                    }
                    None => {
                        ensure!(
                            info.controller.is_none(),
                            "Absent overlay host has a controller"
                        );
                        let mut hosts = self.cards.iter().filter_map(|card| match card.placement {
                            CardPlacement::Overlay { host, .. }
                                if !self.cards.iter().any(|other| other.id == host) =>
                            {
                                Some(host)
                            }
                            _ => None,
                        });
                        let host = hosts.next().context("Absent overlay host not found")?;
                        ensure!(
                            hosts.all(|other| other == host),
                            "Absent overlay host is ambiguous"
                        );
                        host
                    }
                };
                Ok(Some(CardPlacement::Overlay { host, sequence }))
            }
        }
    }

    fn place_moved_card(
        &mut self,
        mut before: Option<Card>,
        current: Option<CardPlacement>,
        code: u32,
        position: Option<CardPositionFlags>,
    ) -> anyhow::Result<()> {
        if let Some(card) = &mut before {
            Self::update_code(card, code)?;
        }
        if let Some(placement) = current {
            let id = match before {
                Some(card) => card.id,
                None => {
                    let id = CardId(self.next_id);
                    self.next_id = self.next_id.checked_add(1).context("Card IDs exhausted")?;
                    id
                }
            };
            let code = if code == 0 {
                before.map_or(0, |card| card.code)
            } else {
                code
            };
            self.place_card(Card {
                id,
                code,
                placement,
                position,
            })?;
        }
        Ok(())
    }

    fn remove_card(&mut self, placement: CardPlacement) -> anyhow::Result<Card> {
        let index = self
            .cards
            .iter()
            .position(|card| card.placement == placement)
            .context("No card at move source")?;
        match placement {
            CardPlacement::Zone(address) if Self::is_pile(address.location) => {
                self.pile_len(address.controller, address.location)?;
            }
            CardPlacement::Overlay { host, .. } => {
                self.material_len(host)?;
            }
            _ => {}
        }
        let removed = self.cards.remove(index);
        for card in &mut self.cards {
            match (&mut card.placement, placement) {
                (CardPlacement::Zone(other), CardPlacement::Zone(address))
                    if Self::is_pile(address.location)
                        && other.controller == address.controller
                        && other.location == address.location
                        && other.sequence > address.sequence =>
                {
                    other.sequence -= 1;
                }
                (
                    CardPlacement::Overlay {
                        host: other_host,
                        sequence: other_sequence,
                    },
                    CardPlacement::Overlay { host, sequence },
                ) if *other_host == host && *other_sequence > sequence => {
                    *other_sequence -= 1;
                }
                _ => {}
            }
        }
        Ok(removed)
    }

    fn place_card(&mut self, card: Card) -> anyhow::Result<()> {
        match card.placement {
            CardPlacement::Zone(address) if Self::is_pile(address.location) => {
                let len = self.pile_len(address.controller, address.location)?;
                ensure!(
                    address.sequence <= len,
                    "Move destination is outside the pile"
                );
                len.checked_add(1).context("Pile sequence overflow")?;
                for other in &mut self.cards {
                    if let CardPlacement::Zone(other) = &mut other.placement {
                        if other.controller == address.controller
                            && other.location == address.location
                            && other.sequence >= address.sequence
                        {
                            other.sequence += 1;
                        }
                    }
                }
            }
            CardPlacement::Zone(address) => {
                ensure!(
                    self.card_at(address).is_none(),
                    "Move destination is occupied"
                );
            }
            CardPlacement::Overlay { host, sequence } => {
                ensure!(
                    self.cards.iter().any(|other| other.id == host
                        && matches!(other.placement, CardPlacement::Zone(_))),
                    "Overlay destination has no host"
                );
                ensure!(
                    !self.cards.iter().any(|other| matches!(other.placement, CardPlacement::Overlay { host, .. } if host == card.id)),
                    "An overlay material cannot retain its own materials"
                );
                let len = self.material_len(host)?;
                ensure!(
                    sequence == len,
                    "Overlay destination must append to materials"
                );
                len.checked_add(1).context("Overlay sequence overflow")?;
            }
        }
        self.cards.push(card);
        Ok(())
    }

    fn pile_len(&self, controller: Player, location: CardLocation) -> anyhow::Result<u32> {
        let mut sequences = self.cards.iter().filter_map(|card| match card.placement {
            CardPlacement::Zone(address)
                if address.controller == controller && address.location == location =>
            {
                Some(address.sequence)
            }
            _ => None,
        });
        let len = u32::try_from(sequences.clone().count()).context("Pile is too large")?;
        ensure!(
            sequences.all(|sequence| sequence < len),
            "Pile sequences are not contiguous"
        );
        Ok(len)
    }

    fn material_len(&self, host: CardId) -> anyhow::Result<u32> {
        let mut sequences = self.cards.iter().filter_map(|card| match card.placement {
            CardPlacement::Overlay {
                host: card_host,
                sequence,
            } if card_host == host => Some(sequence),
            _ => None,
        });
        let len = u32::try_from(sequences.clone().count()).context("Too many overlay materials")?;
        ensure!(
            sequences.all(|sequence| sequence < len),
            "Overlay sequences are not contiguous"
        );
        Ok(len)
    }

    fn is_pile(location: CardLocation) -> bool {
        !matches!(
            location,
            CardLocation::MonsterZone | CardLocation::SpellTrapZone
        )
    }
}

#[cfg(test)]
#[path = "../../tests/unit/ui/card_layer.rs"]
mod tests;
