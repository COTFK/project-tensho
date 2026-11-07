use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32, read_u64};
use ocgcore_ffi::constants::{PHINT_DESC_ADD, PHINT_DESC_REMOVE};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayerHint {
    DescriptionAdd(u64),
    DescriptionRemove(u64),
    Unknown { kind: u8, value: u64 },
}

impl PlayerHint {
    fn from_wire(kind: u8, value: u64) -> Self {
        match u32::from(kind) {
            PHINT_DESC_ADD => Self::DescriptionAdd(value),
            PHINT_DESC_REMOVE => Self::DescriptionRemove(value),
            _ => Self::Unknown { kind, value },
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlayerHintMessageData {
    pub player: Player,
    pub hint: PlayerHint,
}

impl TryFrom<&[u8]> for PlayerHintMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 11, "Invalid player hint message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "player hint player")?)?;
        let kind = read_u8(bytes, &mut cursor, "player hint type")?;
        let value = read_u64(bytes, &mut cursor, "player hint value")?;
        Ok(Self {
            player,
            hint: PlayerHint::from_wire(kind, value),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/player_hint.rs"]
mod tests;
