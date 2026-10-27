#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum Player {
    #[default]
    Player = 0,
    Opponent = 1,
}

impl TryFrom<u8> for Player {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self> {
        match value {
            0 => Ok(Self::Player),
            1 => Ok(Self::Opponent),
            _ => anyhow::bail!("Received invalid player value: {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_actual_players() {
        assert_eq!(Player::try_from(0).unwrap(), Player::Player);
        assert_eq!(Player::try_from(1).unwrap(), Player::Opponent);
        for value in 2..=u8::MAX {
            assert!(Player::try_from(value).is_err());
        }
    }
}
