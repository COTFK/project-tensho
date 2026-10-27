use ocgcore_ffi::constants::PLAYER_NONE;

use super::Player;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Winner {
    Player(Player),
    Draw,
}

impl TryFrom<u8> for Winner {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match u32::from(value) {
            0 | 1 => Ok(Self::Player(Player::try_from(value)?)),
            PLAYER_NONE => Ok(Self::Draw),
            _ => anyhow::bail!("Received invalid winner value: {value}"),
        }
    }
}
