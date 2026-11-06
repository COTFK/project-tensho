use anyhow::{Context, ensure};

use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{CardLocationInfo, CardLocationKind, CardLocationPosition};
use crate::ocgcore::messages::{CoreMessage, DrawMessageData, MoveMessageData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CardId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CardAddress {
    pub controller: Player,
    pub location: CardLocation,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Card {
    pub id: CardId,
    pub code: u32,
    pub address: CardAddress,
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
        self.cards.push(Card { id, code, address });
        Ok(id)
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn card_at(&self, address: CardAddress) -> Option<&Card> {
        self.cards.iter().find(|card| card.address == address)
    }

    pub fn apply(&mut self, message: &CoreMessage) -> anyhow::Result<()> {
        let mut next = self.clone();
        match message {
            CoreMessage::Move(message) => next.apply_move(message)?,
            CoreMessage::Draw(message) => next.apply_draw(message)?,
            _ => anyhow::bail!("Unsupported card update"),
        }
        *self = next;
        Ok(())
    }

    fn apply_move(&mut self, message: &MoveMessageData) -> anyhow::Result<()> {
        let previous = Self::address_from(message.previous)?;
        let current = Self::address_from(message.current)?;
        ensure!(
            previous.is_some() || current.is_some(),
            "Move has no card location"
        );
        self.move_card(previous, current, message.card_code)
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
            self.move_card(Some(previous), Some(current), drawn.code)?;
        }
        Ok(())
    }

    fn address_from(info: CardLocationInfo) -> anyhow::Result<Option<CardAddress>> {
        match info.location {
            CardLocationKind::None => Ok(None),
            CardLocationKind::Overlay { .. } => {
                anyhow::bail!("Overlay moves are not supported yet")
            }
            CardLocationKind::Known(location) => {
                let controller = info.controller.context("Card location has no controller")?;
                let CardLocationPosition::Position(_) = info.position else {
                    anyhow::bail!("Non-overlay card has an overlay sequence");
                };
                Ok(Some(CardAddress {
                    controller,
                    location,
                    sequence: info.sequence,
                }))
            }
        }
    }

    fn move_card(
        &mut self,
        previous: Option<CardAddress>,
        current: Option<CardAddress>,
        code: u32,
    ) -> anyhow::Result<()> {
        let before = previous
            .map(|address| self.remove_card(address))
            .transpose()?;
        if let Some(card) = before {
            ensure!(
                card.code == 0 || code == 0 || card.code == code,
                "Move card code does not match the card at its source"
            );
        }
        if let Some(address) = current {
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
            self.place_card(Card { id, code, address })?;
        }
        Ok(())
    }

    fn remove_card(&mut self, address: CardAddress) -> anyhow::Result<Card> {
        let index = self
            .cards
            .iter()
            .position(|card| card.address == address)
            .context("No card at move source")?;
        if Self::is_pile(address.location) {
            self.pile_len(address.controller, address.location)?;
        }
        let removed = self.cards.remove(index);
        if Self::is_pile(address.location) {
            for card in &mut self.cards {
                if card.address.controller == address.controller
                    && card.address.location == address.location
                    && card.address.sequence > address.sequence
                {
                    card.address.sequence -= 1;
                }
            }
        }
        Ok(removed)
    }

    fn place_card(&mut self, card: Card) -> anyhow::Result<()> {
        let address = card.address;
        if Self::is_pile(address.location) {
            let len = self.pile_len(address.controller, address.location)?;
            ensure!(
                address.sequence <= len,
                "Move destination is outside the pile"
            );
            len.checked_add(1).context("Pile sequence overflow")?;
            for other in &mut self.cards {
                if other.address.controller == address.controller
                    && other.address.location == address.location
                    && other.address.sequence >= address.sequence
                {
                    other.address.sequence += 1;
                }
            }
        } else {
            ensure!(
                self.card_at(address).is_none(),
                "Move destination is occupied"
            );
        }
        self.cards.push(card);
        Ok(())
    }

    fn pile_len(&self, controller: Player, location: CardLocation) -> anyhow::Result<u32> {
        let mut cards = self.cards.iter().filter(|card| {
            card.address.controller == controller && card.address.location == location
        });
        let len = u32::try_from(cards.clone().count()).context("Pile is too large")?;
        ensure!(
            cards.all(|card| card.address.sequence < len),
            "Pile sequences are not contiguous"
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
