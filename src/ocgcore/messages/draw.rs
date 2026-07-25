use crate::ocgcore::CardData;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrawMessageData {
    pub player: u8,
    pub cards: Vec<CardData>,
}

impl TryFrom<&[u8]> for DrawMessageData {
    type Error = anyhow::Error;

    fn try_from(raw_bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = raw_bytes[5];

        let count =
            u32::from_le_bytes([raw_bytes[6], raw_bytes[7], raw_bytes[8], raw_bytes[9]]) as usize;

        let mut cards = Vec::new();

        for index in 0..count {
            let offset = 10 + (index * 8);

            let card_code = u32::from_le_bytes([
                raw_bytes[offset],
                raw_bytes[offset + 1],
                raw_bytes[offset + 2],
                raw_bytes[offset + 3],
            ]);

            let raw_position = u32::from_le_bytes([
                raw_bytes[offset + 4],
                raw_bytes[offset + 5],
                raw_bytes[offset + 6],
                raw_bytes[offset + 7],
            ]);

            cards.push(CardData {
                card_code,
                controller: CardController::try_from(player)?,
                location: CardLocation::Hand,
                position: Some(BattlePosition::try_from(raw_position)?),
                ..Default::default()
            });
        }

        Ok(DrawMessageData { player, cards })
    }
}
