use crate::ocgcore::DuelCardData;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::read_u8;
use crate::ocgcore::utility::read_u32;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SelectUnselectMessageData {
    pub playerid: u8,
    pub finishable: bool,
    pub cancelable: bool,
    pub min_count: u32,
    pub max_count: u32,
    pub select_cards: Vec<DuelCardData>,
    pub unselect_cards: Vec<DuelCardData>,
}

impl SelectUnselectMessageData {
    pub fn response_index_for(&self, card: &DuelCardData) -> Option<u32> {
        if let Some(index) = self.unselect_cards.iter().position(|selectable_card| {
            selectable_card.controller == card.controller
                && selectable_card.location == card.location
                && selectable_card.sequence == card.sequence
                && selectable_card.card_code == card.card_code
        }) {
            return Some((self.select_cards.len() + index) as u32);
        }

        self.select_cards
            .iter()
            .position(|selectable_card| {
                selectable_card.controller == card.controller
                    && selectable_card.location == card.location
                    && selectable_card.sequence == card.sequence
                    && selectable_card.card_code == card.card_code
            })
            .map(|index| index as u32)
    }

    pub fn select_card_for(
        &self,
        controller: Player,
        location: CardLocation,
        sequence: u32,
    ) -> Option<DuelCardData> {
        self.select_cards
            .iter()
            .chain(self.unselect_cards.iter())
            .find(|card| {
                card.controller == controller
                    && card.location == location
                    && u32::from(card.sequence) == sequence
            })
            .copied()
    }
}

impl TryFrom<&[u8]> for SelectUnselectMessageData {
    type Error = anyhow::Error;

    fn try_from(raw_bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let mut cursor = 5;

        let playerid = read_u8(raw_bytes, &mut cursor, "playerid")?;
        let finishable = read_u8(raw_bytes, &mut cursor, "finishable")? != 0;
        let cancelable = read_u8(raw_bytes, &mut cursor, "cancelable")? != 0;
        let min_count = read_u32(raw_bytes, &mut cursor, "min_count")?;
        let max_count = read_u32(raw_bytes, &mut cursor, "max_count")?;
        let selectable_count = read_u32(raw_bytes, &mut cursor, "selectable_count")? as usize;

        anyhow::ensure!(
            selectable_count <= (raw_bytes.len() - cursor) / 14,
            "Truncated selectable cards"
        );

        let mut select_cards = Vec::with_capacity(selectable_count);

        // Selectable cards come first.
        for _ in 0..selectable_count {
            let card_code = read_u32(raw_bytes, &mut cursor, "card_code")?;
            let controller = read_u8(raw_bytes, &mut cursor, "controller")?;
            let location = read_u8(raw_bytes, &mut cursor, "location")?;
            let sequence = read_u32(raw_bytes, &mut cursor, "sequence")?;
            let position = read_u32(raw_bytes, &mut cursor, "position")?;

            select_cards.push(DuelCardData {
                card_code,
                controller: Player::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: BattlePosition::try_from(position).ok(),
                sequence: sequence as u8,
                ..Default::default()
            });
        }

        let unselectable_count = read_u32(raw_bytes, &mut cursor, "unselectable_count")? as usize;

        anyhow::ensure!(
            unselectable_count <= (raw_bytes.len() - cursor) / 14,
            "Truncated unselectable cards"
        );

        let mut unselect_cards = Vec::with_capacity(unselectable_count);

        // Previously selected cards come after the selectable list.
        for _ in 0..unselectable_count {
            let card_code = read_u32(raw_bytes, &mut cursor, "unselect_cards.card_code")?;
            let controller = read_u8(raw_bytes, &mut cursor, "unselect_cards.controller")?;
            let location = read_u8(raw_bytes, &mut cursor, "unselect_cards.location")?;
            let sequence = read_u32(raw_bytes, &mut cursor, "unselect_cards.sequence")?;
            let position = read_u32(raw_bytes, &mut cursor, "unselect_cards.position")?;

            unselect_cards.push(DuelCardData {
                card_code,
                controller: Player::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: BattlePosition::try_from(position).ok(),
                sequence: sequence as u8,
                is_selected: true,
                ..Default::default()
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
