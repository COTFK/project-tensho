#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CardController {
    #[default]
    Player = 0,
    _Opponent = 1,
}

impl TryFrom<u8> for CardController {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            0 => Ok(CardController::Player),
            1 => Ok(CardController::_Opponent),
            _ => anyhow::bail!("Received wrong controller value"),
        }
    }
}
