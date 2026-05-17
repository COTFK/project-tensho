#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardLocation {
    Deck = 1,
    Hand = 2,
}

impl TryFrom<u8> for CardLocation {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(CardLocation::Deck),
            2 => Ok(CardLocation::Hand),
            _ => anyhow::bail!("Received wrong location value")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            _ => anyhow::bail!("Received wrong owner value")
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardController {
    Player = 0,
    _Opponent = 1,
}

impl TryFrom<u8> for CardController {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            0 => Ok(CardController::Player),
            1 => Ok(CardController::_Opponent),
            _ => anyhow::bail!("Received wrong controller value")
        }
    }
}
