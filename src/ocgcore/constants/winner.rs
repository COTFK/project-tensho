use ocgcore_ffi::constants::PLAYER_NONE;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Winner {
    Player,
    Opponent,
    Draw,
}

impl TryFrom<u8> for Winner {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match u32::from(value) {
            0 => Ok(Self::Player),
            1 => Ok(Self::Opponent),
            PLAYER_NONE => Ok(Self::Draw),
            _ => anyhow::bail!("Received invalid winner value: {value}"),
        }
    }
}
