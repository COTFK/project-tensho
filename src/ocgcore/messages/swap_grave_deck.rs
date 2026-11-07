use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SwapGraveDeckMessageData {
    pub player: Player,
    pub extra_insert_index: u32,
    pub extra_cards: Vec<u8>,
}

impl TryFrom<&[u8]> for SwapGraveDeckMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "exchange player")?)?;
        let extra_insert_index = read_u32(bytes, &mut cursor, "extra-deck insertion index")?;
        let bitmap_length = read_u32(bytes, &mut cursor, "extra-card bitmap length")?;
        let expected = bitmap_length
            .checked_add(10)
            .context("Exchange message length overflow")?;
        anyhow::ensure!(
            length == expected,
            "Invalid exchange message length: {length}, expected {expected}"
        );
        let end = cursor
            .checked_add(usize::try_from(bitmap_length)?)
            .context("Exchange bitmap length overflow")?;
        let extra_cards = bytes
            .get(cursor..end)
            .context("Truncated exchange bitmap")?
            .to_vec();
        Ok(Self {
            player,
            extra_insert_index,
            extra_cards,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/swap_grave_deck.rs"]
mod tests;
