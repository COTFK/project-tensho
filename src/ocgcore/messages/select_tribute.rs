use crate::ocgcore::CardData;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectTributeMessageData {
    pub player: u8,
    pub is_cancelable: bool,
    pub min_select: u32,
    pub max_select: u32,
    pub candidates: Vec<CardData>,
}

impl TryFrom<&[u8]> for SelectTributeMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[5];
        let is_cancelable = bytes[6] != 0;

        let min_select = u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]);
        let max_select = u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]);
        let count = u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]) as usize;

        let mut candidates = Vec::with_capacity(count);

        for index in 0..count {
            let offset = 19 + (index * 11);

            let card_code = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            let controller = bytes[offset + 4];
            let location = bytes[offset + 5];
            let sequence = u32::from_le_bytes([
                bytes[offset + 6],
                bytes[offset + 7],
                bytes[offset + 8],
                bytes[offset + 9],
            ]) as u8;
            let _release_param = bytes[offset + 10];

            candidates.push(CardData {
                card_code,
                controller: CardController::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: None,
                description: None,
                is_selected: false,
                sequence,
                action_index: Some(index as u8),
            });
        }

        Ok(SelectTributeMessageData {
            player,
            is_cancelable,
            min_select,
            max_select,
            candidates,
        })
    }
}
