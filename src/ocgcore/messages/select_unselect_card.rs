use crate::ocgcore::ActiveCard;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::utility::read_u8;
use crate::ocgcore::utility::read_u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectUnselectMessage {
    pub playerid: u8,
    pub finishable: bool,
    pub cancelable: bool,
    pub min_count: u32,
    pub max_count: u32,
    pub select_cards: Vec<ActiveCard>,
    pub unselect_cards: Vec<ActiveCard>,
}

impl SelectUnselectMessage {
    pub fn response_index_for(&self, card: &ActiveCard) -> Option<u32> {
        if let Some(index) = self.unselect_cards.iter().position(|selectable_card| {
            selectable_card.location == card.location
                && selectable_card.sequence == card.sequence
                && selectable_card.card_code == card.card_code
        }) {
            return Some((self.select_cards.len() + index) as u32);
        }

        self.select_cards
            .iter()
            .position(|selectable_card| {
                selectable_card.location == card.location
                    && selectable_card.sequence == card.sequence
                    && selectable_card.card_code == card.card_code
            })
            .map(|index| index as u32)
    }

    pub fn select_card_for(&self, location: CardLocation, index: u8) -> Option<ActiveCard> {
        self.select_cards
            .iter()
            .chain(self.unselect_cards.iter())
            .find(|card| card.location == location && card.sequence == index)
            .copied()
    }
}

impl TryFrom<&[u8]> for SelectUnselectMessage {
    type Error = anyhow::Error;

    fn try_from(raw_bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        // 1. Handle length headers if the socket layer prepends them
        let message_offset = if raw_bytes[0] == 26 {
            0
        } else if raw_bytes.len() >= 5 && raw_bytes[4] == 26 {
            let declared_len = u32::from_le_bytes(raw_bytes[0..4].try_into().unwrap()) as usize;
            if declared_len != raw_bytes.len().saturating_sub(4) {
                anyhow::bail!("Length prefix mismatch parsing message 26");
            }
            4
        } else {
            anyhow::bail!(
                "Invalid message ID: Expected MSG_SELECT_UNSELECT_CARD (26), got {}",
                raw_bytes.first().copied().unwrap_or_default()
            );
        };

        let mut cursor = message_offset + 1;

        let playerid = read_u8(raw_bytes, &mut cursor, "playerid")?;
        let finishable = read_u8(raw_bytes, &mut cursor, "finishable")? != 0;
        let cancelable = read_u8(raw_bytes, &mut cursor, "cancelable")? != 0;
        let min_count = read_u32(raw_bytes, &mut cursor, "min_count")?;
        let max_count = read_u32(raw_bytes, &mut cursor, "max_count")?;
        let selectable_count = read_u32(raw_bytes, &mut cursor, "selectable_count")? as usize;

        // Sanity guard against massive/corrupted allocations
        if selectable_count > 100 {
            anyhow::bail!("Impossible selection count detected: {}", selectable_count);
        }

        let mut select_cards = Vec::with_capacity(selectable_count);

        // Selectable cards come first.
        for _ in 0..selectable_count {
            let card_code = read_u32(raw_bytes, &mut cursor, "card_code")?;
            let controller = read_u8(raw_bytes, &mut cursor, "controller")?;
            let location = read_u8(raw_bytes, &mut cursor, "location")?;
            let sequence = read_u32(raw_bytes, &mut cursor, "sequence")?;
            let position = read_u32(raw_bytes, &mut cursor, "position")?;

            select_cards.push(ActiveCard {
                card_code,
                controller: CardController::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: BattlePosition::try_from(position).ok(),
                sequence: sequence as u8,
                action_index: None,
                description: None,
                is_selected: false,
            });
        }

        let unselectable_count = read_u32(raw_bytes, &mut cursor, "unselectable_count")? as usize;

        if unselectable_count > 100 {
            anyhow::bail!(
                "Impossible unselectable count detected: {}",
                unselectable_count
            );
        }

        let mut unselect_cards = Vec::with_capacity(unselectable_count);

        // Previously selected cards come after the selectable list.
        for _ in 0..unselectable_count {
            let card_code = read_u32(raw_bytes, &mut cursor, "unselect_cards.card_code")?;
            let controller = read_u8(raw_bytes, &mut cursor, "unselect_cards.controller")?;
            let location = read_u8(raw_bytes, &mut cursor, "unselect_cards.location")?;
            let sequence = read_u32(raw_bytes, &mut cursor, "unselect_cards.sequence")?;
            let position = read_u32(raw_bytes, &mut cursor, "unselect_cards.position")?;

            unselect_cards.push(ActiveCard {
                card_code,
                controller: CardController::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: BattlePosition::try_from(position).ok(),
                sequence: sequence as u8,
                action_index: None,
                description: None,
                is_selected: true,
            });
        }

        Ok(Self {
            playerid,
            finishable,
            cancelable,
            min_count,
            max_count,
            select_cards,
            unselect_cards,
        })
    }
}
