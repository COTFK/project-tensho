#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum Player {
    #[default]
    Local = 0,
    Opponent = 1,
}

impl TryFrom<u8> for Player {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self> {
        match value {
            0 => Ok(Self::Local),
            1 => Ok(Self::Opponent),
            _ => anyhow::bail!("Received invalid player value: {value}"),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/constants/player.rs"]
mod tests;
