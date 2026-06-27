use crate::ocgcore::CardData;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SelectCardMessageData {
    pub min_select: u32,
    pub max_select: u32,
    pub cards: Vec<CardData>,
}

impl TryFrom<&[u8]> for SelectCardMessageData {
    type Error = anyhow::Error;

    fn try_from(raw_bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let min_select =
            u32::from_le_bytes([raw_bytes[7], raw_bytes[8], raw_bytes[9], raw_bytes[10]]);
        let max_select =
            u32::from_le_bytes([raw_bytes[11], raw_bytes[12], raw_bytes[13], raw_bytes[14]]);
        let count = u32::from_le_bytes([raw_bytes[15], raw_bytes[16], raw_bytes[17], raw_bytes[18]])
            as usize;

        let mut cards = Vec::new();

        // The sequential payload array of card structures starts exactly at index 19
        for index in 0..count {
            let offset = 19 + (index * 14);

            if offset + 14 > raw_bytes.len() {
                panic!(
                    "Truncated packet encountered parsing card block index {}",
                    index
                );
            }

            // 1. 4-byte Card ID
            let card_code = u32::from_le_bytes([
                raw_bytes[offset],
                raw_bytes[offset + 1],
                raw_bytes[offset + 2],
                raw_bytes[offset + 3],
            ]);

            // 2. Controller Alignment
            let controller_byte = raw_bytes[offset + 4];
            let controller = if controller_byte == 0 {
                CardController::Player
            } else {
                CardController::_Opponent
            };

            // 3. Bitmask Location Extraction
            let raw_location_byte = raw_bytes[offset + 5];
            let _is_xyz_material = (raw_location_byte & 0x80) != 0;
            let base_location_byte = raw_location_byte & 0x7F;

            let location = CardLocation::try_from(base_location_byte)?;

            // 4. Sequence placement
            let sequence = raw_bytes[offset + 6];

            cards.push(CardData {
                card_code,
                controller,
                location,
                sequence,
                ..Default::default()
            });
        }

        Ok(Self {
            min_select,
            max_select,
            cards,
        })
    }
}
