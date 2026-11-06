use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShuffleHandMessageData {
    pub player: Player,
    pub card_codes: Vec<u32>,
}

impl TryFrom<&[u8]> for ShuffleHandMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "shuffle player")?)?;
        let count = read_u32(bytes, &mut cursor, "shuffled card count")?;
        let expected_length = count
            .checked_mul(4)
            .and_then(|size| size.checked_add(6))
            .context("Shuffled card count exceeds message size")?;
        anyhow::ensure!(
            length == expected_length,
            "Invalid shuffle hand message length: {length}, expected {expected_length}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Shuffle hand message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated shuffle hand message");

        let mut card_codes = Vec::with_capacity(count as usize);
        for _ in 0..count {
            card_codes.push(read_u32(bytes, &mut cursor, "shuffled card code")?);
        }
        Ok(Self { player, card_codes })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/shuffle_hand.rs"]
mod tests;
