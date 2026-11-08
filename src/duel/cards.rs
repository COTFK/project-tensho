use anyhow::{Context, ensure};
use ocgcore_ffi::constants::{MSG_RELOAD_FIELD, MSG_TAG_SWAP};

use crate::ocgcore::DuelCardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{
    CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
};
use crate::ocgcore::messages::{
    ConfirmCardsMessageData, CoreMessage, DeckTopMessageData, DrawMessageData, MoveMessageData,
    RemoveCardsMessageData, ShuffleSetCardMessageData, SwapGraveDeckMessageData, SwapMessageData,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CardInstanceId(u64);

impl std::fmt::Display for CardInstanceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CardAddress {
    pub controller: Player,
    pub location: CardLocation,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardPlacement {
    Zone(CardAddress),
    Overlay { host: CardInstanceId, sequence: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrackedCard {
    pub id: CardInstanceId,
    pub code: u32,
    pub placement: CardPlacement,
    pub position: Option<CardPositionFlags>,
    pub data: Option<DuelCardData>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CardTracker {
    cards: Vec<TrackedCard>,
    next_id: u64,
}

impl CardTracker {
    pub fn from_decks(main_deck: &[u32], extra_deck: &[u32]) -> anyhow::Result<Self> {
        let mut tracker = Self::default();
        for (location, codes) in [
            (CardLocation::Deck, main_deck),
            (CardLocation::ExtraDeck, extra_deck),
        ] {
            for (sequence, &code) in codes.iter().enumerate() {
                tracker.insert(
                    CardAddress {
                        controller: Player::Local,
                        location,
                        sequence: sequence as u32,
                    },
                    code,
                )?;
            }
        }
        Ok(tracker)
    }

    pub fn insert(&mut self, address: CardAddress, code: u32) -> anyhow::Result<CardInstanceId> {
        ensure!(
            self.card_at(address).is_none(),
            "Card address is already occupied"
        );
        let id = self.allocate_id();
        self.cards.push(TrackedCard {
            id,
            code,
            placement: CardPlacement::Zone(address),
            position: None,
            data: None,
        });
        Ok(id)
    }

    pub fn cards(&self) -> &[TrackedCard] {
        &self.cards
    }

    pub fn card_at(&self, address: CardAddress) -> Option<&TrackedCard> {
        self.cards
            .iter()
            .find(|card| card.placement == CardPlacement::Zone(address))
    }

    pub fn card_by_id(&self, id: CardInstanceId) -> Option<&TrackedCard> {
        self.cards.iter().find(|card| card.id == id)
    }

    pub fn card_at_location(&self, info: CardLocationInfo) -> anyhow::Result<Option<&TrackedCard>> {
        let Some(placement) = self.placement_from(info)? else {
            return Ok(None);
        };
        Ok(self.cards.iter().find(|card| card.placement == placement))
    }

    pub fn update_queried_data(
        &mut self,
        address: CardAddress,
        data: DuelCardData,
    ) -> anyhow::Result<()> {
        let card = self
            .cards
            .iter_mut()
            .find(|card| card.placement == CardPlacement::Zone(address))
            .with_context(|| format!("No tracked card for queried data at {address:?}"))?;
        if address.location == CardLocation::Graveyard {
            Self::update_code_if_known(card, data.card_code);
        }
        card.data = Some(data);
        Ok(())
    }

    pub fn zone_data(&self, location: CardLocation) -> Option<Vec<Option<DuelCardData>>> {
        let mut data = Vec::new();
        for card in &self.cards {
            let CardPlacement::Zone(address) = card.placement else {
                continue;
            };
            if address.controller != Player::Local || address.location != location {
                continue;
            }
            let index = usize::try_from(address.sequence).ok()?;
            if data.len() <= index {
                data.resize(index + 1, None);
            }
            data[index] = Some(card.data?);
        }
        Some(data)
    }

    pub fn apply(&mut self, message: &CoreMessage) -> anyhow::Result<()> {
        match message {
            CoreMessage::Move(message) => self.apply_move(message)?,
            CoreMessage::Swap(message) => self.apply_swap(message)?,
            CoreMessage::SwapGraveDeck(message) => self.swap_grave_deck(message)?,
            CoreMessage::RemoveCards(message) => self.apply_remove_cards(message)?,
            CoreMessage::Draw(message) => self.apply_draw(message)?,
            CoreMessage::ReverseDeck => self.reverse_decks(),
            CoreMessage::DeckTop(message) => self.apply_deck_top(message)?,
            CoreMessage::ConfirmDeckTop(message) => {
                self.confirm_top_cards(message, CardLocation::Deck)?
            }
            CoreMessage::ConfirmExtraTop(message) => {
                self.confirm_top_cards(message, CardLocation::ExtraDeck)?
            }
            CoreMessage::ShuffleDeck(message) => {
                self.apply_shuffle(message.player, CardLocation::Deck, None)?
            }
            CoreMessage::ShuffleHand(message) => self.apply_shuffle(
                message.player,
                CardLocation::Hand,
                Some(&message.card_codes),
            )?,
            CoreMessage::ShuffleExtra(message) => self.apply_shuffle(
                message.player,
                CardLocation::ExtraDeck,
                Some(&message.card_codes),
            )?,
            CoreMessage::ShuffleSetCard(message) => self.apply_set_shuffle(message)?,
            CoreMessage::PositionChange(message) => self.update_appearance(
                CardPlacement::Zone(CardAddress {
                    controller: message.controller,
                    location: message.location,
                    sequence: message.sequence,
                }),
                message.card_code,
                message.current_position,
            )?,
            CoreMessage::SetCard(message) => {
                self.apply_card_event(message.location, message.card_code)?
            }
            CoreMessage::Summoning(message)
            | CoreMessage::SpecialSummoning(message)
            | CoreMessage::FlipSummoning(message) => {
                self.apply_card_event(message.location, message.card_code)?
            }
            CoreMessage::Unsupported(kind)
                if matches!(u32::from(*kind), MSG_TAG_SWAP | MSG_RELOAD_FIELD) =>
            {
                anyhow::bail!("Unsupported structural core message {kind}");
            }
            _ => {}
        }
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
        self.place_moved_card(card, current, message.card_code, position);
        Ok(())
    }

    fn apply_swap(&mut self, message: &SwapMessageData) -> anyhow::Result<()> {
        let (first, first_placement) =
            self.prepare_swap_source(message.first_location, message.first_code)?;
        let (second, second_placement) =
            self.prepare_swap_source(message.second_location, message.second_code)?;
        self.cards[first].placement = second_placement;
        self.cards[second].placement = first_placement;
        Ok(())
    }

    fn swap_grave_deck(&mut self, message: &SwapGraveDeckMessageData) -> anyhow::Result<()> {
        let player = message.player;
        let grave_len = self.pile_len(player, CardLocation::Graveyard);
        let extra_len = self.pile_len(player, CardLocation::ExtraDeck);
        ensure!(
            message.extra_insert_index <= extra_len,
            "Extra-deck insertion index is outside the pile"
        );
        let bitmap_len = (grave_len as usize).div_ceil(8);
        ensure!(
            message.extra_cards.len() == bitmap_len,
            "Exchange bitmap does not match the graveyard size"
        );
        if grave_len % 8 != 0 {
            let valid_bits = (1u16 << (grave_len % 8)) - 1;
            ensure!(
                u16::from(message.extra_cards[bitmap_len - 1]) & !valid_bits == 0,
                "Exchange bitmap contains out-of-range cards"
            );
        }
        let redirected = message
            .extra_cards
            .iter()
            .map(|byte| byte.count_ones())
            .sum::<u32>();
        if grave_len - redirected == 1 {
            let surviving_sequence = (0..grave_len)
                .find(|sequence| {
                    message.extra_cards[*sequence as usize / 8] & (1 << (*sequence % 8)) == 0
                })
                .context("Missing surviving deck card")?;
            ensure!(
                surviving_sequence == 0,
                "Cannot track exchange: the core retains a stale sequence on the only deck card"
            );
        }
        let grave_indices: Vec<usize> = (0..grave_len)
            .map(|sequence| {
                self.cards
                    .iter()
                    .position(|card| {
                        card.placement
                            == CardPlacement::Zone(CardAddress {
                                controller: player,
                                location: CardLocation::Graveyard,
                                sequence,
                            })
                    })
                    .context("Missing card in exchanged graveyard")
            })
            .collect::<anyhow::Result<_>>()?;
        for card in &mut self.cards {
            if let CardPlacement::Zone(address) = &mut card.placement {
                if address.controller == player && address.location == CardLocation::Deck {
                    address.location = CardLocation::Graveyard;
                    card.position = Some(
                        CardPositionFlags::FACE_UP_ATTACK | CardPositionFlags::FACE_UP_DEFENSE,
                    );
                } else if address.controller == player
                    && address.location == CardLocation::ExtraDeck
                    && address.sequence >= message.extra_insert_index
                {
                    address.sequence += redirected;
                }
            }
        }
        let mut deck_sequence = 0;
        let mut extra_sequence = message.extra_insert_index;
        for (sequence, index) in grave_indices.into_iter().enumerate() {
            let is_extra = message.extra_cards[sequence as usize / 8] & (1 << (sequence % 8)) != 0;
            let (location, destination) = if is_extra {
                let destination = extra_sequence;
                extra_sequence += 1;
                (CardLocation::ExtraDeck, destination)
            } else {
                let destination = deck_sequence;
                deck_sequence += 1;
                (CardLocation::Deck, destination)
            };
            self.cards[index].placement = CardPlacement::Zone(CardAddress {
                controller: player,
                location,
                sequence: destination,
            });
            self.cards[index].position = Some(CardPositionFlags::FACE_DOWN_DEFENSE);
        }
        Ok(())
    }

    fn apply_remove_cards(&mut self, message: &RemoveCardsMessageData) -> anyhow::Result<()> {
        // Full chunks can be continuations of an unmarked, larger removal group.
        ensure!(
            message.cards.len() < 255,
            "Cannot resolve a full removal chunk without knowing its group boundaries"
        );
        let mut ids = Vec::with_capacity(message.cards.len());
        for &info in &message.cards {
            ensure!(
                !matches!(info.location, CardLocationKind::Overlay { .. }),
                "Direct overlay removal does not remove the material from the core"
            );
            let placement = self
                .placement_from(info)?
                .context("Removal target has no location")?;
            let id = self
                .cards
                .iter()
                .find(|card| card.placement == placement)
                .context("No card at removal location")?
                .id;
            ensure!(!ids.contains(&id), "Duplicate removal target");
            ids.push(id);
        }
        for id in ids {
            let placement = self
                .cards
                .iter()
                .find(|card| card.id == id)
                .context("Removal target disappeared")?
                .placement;
            self.remove_card(placement)?;
        }
        Ok(())
    }

    fn prepare_swap_source(
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
        Self::update_code_if_known(card, code);
        card.position = Some(position);
        Ok((index, placement))
    }

    fn apply_draw(&mut self, message: &DrawMessageData) -> anyhow::Result<()> {
        for drawn in &message.cards {
            let deck_len = self.pile_len(message.player, CardLocation::Deck);
            let hand_len = self.pile_len(message.player, CardLocation::Hand);
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
            );
        }
        Ok(())
    }

    fn reverse_decks(&mut self) {
        for player in [Player::Local, Player::Opponent] {
            let len = self.pile_len(player, CardLocation::Deck);
            for card in &mut self.cards {
                if let CardPlacement::Zone(address) = &mut card.placement {
                    if address.controller == player && address.location == CardLocation::Deck {
                        address.sequence = len - 1 - address.sequence;
                    }
                }
            }
        }
    }

    fn apply_deck_top(&mut self, message: &DeckTopMessageData) -> anyhow::Result<()> {
        let sequence = self
            .pile_len(message.player, CardLocation::Deck)
            .checked_sub(1)
            .and_then(|top| top.checked_sub(message.offset))
            .context("Deck top offset is outside the deck")?;
        self.update_appearance(
            CardPlacement::Zone(CardAddress {
                controller: message.player,
                location: CardLocation::Deck,
                sequence,
            }),
            message.card_code,
            message.position,
        )
    }

    fn confirm_top_cards(
        &mut self,
        message: &ConfirmCardsMessageData,
        location: CardLocation,
    ) -> anyhow::Result<()> {
        for confirmed in &message.cards {
            let address = CardAddress {
                controller: message.player,
                location,
                sequence: confirmed.sequence,
            };
            let card = self
                .cards
                .iter_mut()
                .find(|card| card.placement == CardPlacement::Zone(address))
                .context("No card at confirmed top location")?;
            Self::update_code_if_known(card, confirmed.card_code);
        }
        Ok(())
    }

    fn apply_shuffle(
        &mut self,
        player: Player,
        location: CardLocation,
        codes: Option<&[u32]>,
    ) -> anyhow::Result<()> {
        let len = self.pile_len(player, location);
        if let Some(codes) = codes {
            ensure!(
                codes.len() == len as usize,
                "Shuffle card count does not match the tracked pile"
            );
        }
        let cards: Vec<TrackedCard> = (0..len)
            .map(|sequence| {
                self.card_at(CardAddress {
                    controller: player,
                    location,
                    sequence,
                })
                .copied()
                .context("Missing card in shuffled pile")
            })
            .collect::<anyhow::Result<_>>()?;
        let mut shuffled_len = cards.len();
        if location == CardLocation::ExtraDeck {
            let face_up = CardPositionFlags::FACE_UP_ATTACK | CardPositionFlags::FACE_UP_DEFENSE;
            let face_down =
                CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE;
            if let Some(index) = cards.iter().position(|card| {
                card.position.is_some_and(|position| {
                    position.intersects(face_up) && !position.intersects(face_down)
                })
            }) {
                shuffled_len = index;
            }
        }
        if shuffled_len > 1 {
            ensure!(
                !self.cards.iter().any(|card| matches!(
                    card.placement,
                    CardPlacement::Overlay { host, .. }
                        if cards[..shuffled_len].iter().any(|shuffled| shuffled.id == host)
                )),
                "Cannot shuffle a card with overlay materials"
            );
        }
        for (sequence, old) in cards.iter().enumerate() {
            let code = codes.map(|codes| codes[sequence]);
            let replacement = if sequence < shuffled_len && shuffled_len > 1 {
                Some(self.allocate_id())
            } else {
                None
            };
            let card = self
                .cards
                .iter_mut()
                .find(|card| card.id == old.id)
                .context("Shuffled card disappeared")?;
            if let Some(id) = replacement {
                card.id = id;
                card.code = code.unwrap_or(0);
                card.position = None;
            } else if let Some(code) = code {
                Self::update_code_if_known(card, code);
            }
        }
        Ok(())
    }

    fn apply_set_shuffle(&mut self, message: &ShuffleSetCardMessageData) -> anyhow::Result<()> {
        ensure!(
            message.previous.len() == message.current.len(),
            "Set-card shuffle location counts do not match"
        );
        let mut sources: Vec<(TrackedCard, CardAddress, CardPositionFlags)> =
            Vec::with_capacity(message.previous.len());
        for &info in &message.previous {
            let (address, position) = self.resolve_set_shuffle_location(info)?;
            let card = *self
                .card_at(address)
                .context("Set-card shuffle source is empty")?;
            sources.push((card, address, position));
        }

        let mut destinations = vec![None; sources.len()];
        let mut used = Vec::new();
        for (index, info) in message.current.iter().enumerate() {
            if let Some(info) = info {
                let (address, _) = self.resolve_set_shuffle_location(*info)?;
                destinations[index] = Some(address);
                used.push(address);
            }
        }
        let available: Vec<_> = sources
            .iter()
            .map(|(_, address, _)| *address)
            .filter(|address| !used.contains(address))
            .collect();
        let hidden: Vec<_> = destinations
            .iter()
            .enumerate()
            .filter_map(|(index, destination)| destination.is_none().then_some(index))
            .collect();
        if hidden.len() > 1 {
            ensure!(
                !self.cards.iter().any(|card| matches!(
                    card.placement,
                    CardPlacement::Overlay { host, .. }
                        if hidden.iter().any(|&index| sources[index].0.id == host)
                )),
                "Concealed set-card shuffle destination has overlay materials"
            );
        }
        let hidden_position = hidden
            .first()
            .map(|&index| sources[index].2)
            .filter(|position| hidden.iter().all(|&index| sources[index].2 == *position));
        for (&index, &address) in hidden.iter().zip(&available) {
            destinations[index] = Some(address);
        }
        for (index, (old, _, position)) in sources.iter().enumerate() {
            let replacement = if message.current[index].is_none() && hidden.len() > 1 {
                Some(self.allocate_id())
            } else {
                None
            };
            let card = self
                .cards
                .iter_mut()
                .find(|card| card.id == old.id)
                .context("Set-card shuffle source disappeared")?;
            card.placement = CardPlacement::Zone(
                destinations[index].context("Set-card shuffle destination is missing")?,
            );
            if let Some(id) = replacement {
                card.id = id;
                card.code = 0;
                card.position = hidden_position;
            } else {
                card.position = Some(*position);
            }
        }
        Ok(())
    }

    fn resolve_set_shuffle_location(
        &self,
        info: CardLocationInfo,
    ) -> anyhow::Result<(CardAddress, CardPositionFlags)> {
        let placement = self
            .placement_from(info)?
            .context("Set-card shuffle location is absent")?;
        let CardPlacement::Zone(address) = placement else {
            anyhow::bail!("Set-card shuffle targets an overlay material");
        };
        let CardLocationPosition::Position(position) = info.position else {
            anyhow::bail!("Set-card shuffle has no position");
        };
        Ok((address, position))
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
        Self::update_code_if_known(card, code);
        card.position = Some(position);
        Ok(())
    }

    fn update_code_if_known(card: &mut TrackedCard, code: u32) {
        if code != 0 {
            card.code = code;
        }
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
        before: Option<TrackedCard>,
        current: Option<CardPlacement>,
        code: u32,
        position: Option<CardPositionFlags>,
    ) {
        if let Some(placement) = current {
            let id = match before {
                Some(card) => card.id,
                None => self.allocate_id(),
            };
            let code = if code == 0 {
                before.map_or(0, |card| card.code)
            } else {
                code
            };
            self.place_card(TrackedCard {
                id,
                code,
                placement,
                position,
                data: before.and_then(|card| card.data),
            });
        }
    }

    fn remove_card(&mut self, placement: CardPlacement) -> anyhow::Result<TrackedCard> {
        let index = self
            .cards
            .iter()
            .position(|card| card.placement == placement)
            .context("No card at move source")?;
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

    fn place_card(&mut self, card: TrackedCard) {
        if let CardPlacement::Zone(address) = card.placement {
            if Self::is_pile(address.location) {
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
        }
        self.cards.push(card);
    }

    fn pile_len(&self, controller: Player, location: CardLocation) -> u32 {
        let sequences = self.cards.iter().filter_map(|card| match card.placement {
            CardPlacement::Zone(address)
                if address.controller == controller && address.location == location =>
            {
                Some(address.sequence)
            }
            _ => None,
        });
        sequences.count() as u32
    }

    fn is_pile(location: CardLocation) -> bool {
        !matches!(
            location,
            CardLocation::MonsterZone | CardLocation::SpellTrapZone
        )
    }

    fn allocate_id(&mut self) -> CardInstanceId {
        let id = CardInstanceId(self.next_id);
        self.next_id += 1;
        id
    }
}

#[cfg(test)]
#[path = "../../tests/unit/duel/cards.rs"]
mod tests;
