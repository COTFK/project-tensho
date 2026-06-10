#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardOwner {
    Player = 0,
    _Opponent = 1,
}

impl TryFrom<u8> for CardOwner {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            0 => Ok(CardOwner::Player),
            1 => Ok(CardOwner::_Opponent),
            _ => anyhow::bail!("Received wrong owner value"),
        }
    }
}
