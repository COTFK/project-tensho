use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct RandomSelectedMessageData {
    pub player: Player,
    pub cards: Vec<CardLocationInfo>,
}

impl TryFrom<&[u8]> for RandomSelectedMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "random selected player")?)?;
        let count = read_u32(bytes, &mut cursor, "random selected card count")?;
        let expected_length = count
            .checked_mul(10)
            .and_then(|size| size.checked_add(6))
            .context("Random selected card count exceeds message size")?;
        anyhow::ensure!(
            length == expected_length,
            "Invalid random selected message length: {length}, expected {expected_length}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Random selected message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated random selected message");

        let mut cards = Vec::with_capacity(count as usize);
        for _ in 0..count {
            cards.push(CardLocationInfo::read(bytes, &mut cursor)?);
        }
        Ok(Self { player, cards })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/random_selected.rs"]
mod tests;
