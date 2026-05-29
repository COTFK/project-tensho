use crate::ocgcore::CardData;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortCardMessageData {
    pub player: u8,
    pub cards: Vec<CardData>,
}

impl TryFrom<&[u8]> for SortCardMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[5];
        let count = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        let mut cards = Vec::with_capacity(count);

        for index in 0..count {
            let offset = 10 + (index * 13);
            let card_code = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            let controller = bytes[offset + 4];
            let location = bytes[offset + 5];
            let sequence = bytes[offset + 6];

            cards.push(CardData {
                action_index: None,
                card_code,
                controller: CardController::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: None,
                sequence,
                description: None,
                is_selected: false,
            });
        }

        Ok(SortCardMessageData { player, cards })
    }
}
