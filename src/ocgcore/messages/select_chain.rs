use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::data::CardData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectChainMessageData {
    pub player: u8,
    pub effects: Vec<CardData>,
}

impl TryFrom<&[u8]> for SelectChainMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[6];
        let count = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
        let mut effects = Vec::new();

        let mut offset = 20;
        for index in 0..count {
            if offset + 7 <= bytes.len() {
                let card_code = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);

                let controller = CardController::try_from(bytes[offset + 4])?;
                let location = CardLocation::try_from(bytes[offset + 5])?;
                let sequence = bytes[offset + 6];

                effects.push(CardData {
                    action_index: Some(index as u8),
                    card_code,
                    controller,
                    location,
                    position: None,
                    sequence,
                    description: None,
                    is_selected: false,
                })
            }

            // Advance by 23 bytes to cleanly clear the trailing descriptive payload fields
            offset += 23;
        }

        Ok(SelectChainMessageData { player, effects })
    }
}
