use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeckTopMessageData {
    pub player: Player,
    pub offset: u32,
    pub card_code: u32,
    pub position: CardPositionFlags,
}

impl TryFrom<&[u8]> for DeckTopMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 14, "Invalid deck top message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "deck player")?)?;
        let offset = read_u32(bytes, &mut cursor, "deck top offset")?;
        let card_code = read_u32(bytes, &mut cursor, "deck top card code")?;
        let position = CardPositionFlags::from_bits_retain(read_u32(
            bytes,
            &mut cursor,
            "deck top card position",
        )?);
        Ok(Self {
            player,
            offset,
            card_code,
            position,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/deck_top.rs"]
mod tests;
