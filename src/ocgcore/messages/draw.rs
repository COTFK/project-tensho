use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrawCard {
    pub code: u32,
    pub position: CardPositionFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrawMessageData {
    pub player: Player,
    pub cards: Vec<DrawCard>,
}

impl TryFrom<&[u8]> for DrawMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "draw player")?)?;
        let count = read_u32(bytes, &mut cursor, "draw count")?;
        let expected = count
            .checked_mul(8)
            .and_then(|size| size.checked_add(6))
            .context("Draw count exceeds message size")?;
        anyhow::ensure!(
            length == expected,
            "Invalid draw message length: {length}, expected {expected}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Draw message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated draw message");

        let mut cards = Vec::with_capacity(count as usize);
        for _ in 0..count {
            cards.push(DrawCard {
                code: read_u32(bytes, &mut cursor, "draw card code")?,
                position: CardPositionFlags::from_bits_retain(read_u32(
                    bytes,
                    &mut cursor,
                    "draw card position",
                )?),
            });
        }
        Ok(Self { player, cards })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/draw.rs"]
mod tests;
