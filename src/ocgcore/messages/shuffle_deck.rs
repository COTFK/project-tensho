use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShuffleDeckMessageData {
    pub player: Player,
}

impl TryFrom<&[u8]> for ShuffleDeckMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 2, "Invalid shuffle deck message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "shuffle player")?)?;
        Ok(Self { player })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/shuffle_deck.rs"]
mod tests;
