#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardLocation {
    Deck = 1,
    Hand = 2,
    MonsterZone = 4,
    SpellTrapZone = 8
}

impl TryFrom<u8> for CardLocation {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(CardLocation::Deck),
            2 => Ok(CardLocation::Hand),
            4 => Ok(CardLocation::MonsterZone),
            _ => anyhow::bail!("Received wrong location value: {value}")
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoreMessage {
    Retry = 1,
    Idle = 11,
    SelectEffectYN = 12,
    SelectCard = 15,
    SelectChain = 16,
    SelectPlace = 18
}

impl TryFrom<u8> for CoreMessage {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(CoreMessage::Retry),
            11 => Ok(CoreMessage::Idle),
            12 => Ok(CoreMessage::SelectEffectYN),
            15 => Ok(CoreMessage::SelectCard),
            16 => Ok(CoreMessage::SelectChain),
            18 => Ok(CoreMessage::SelectPlace),
            _ => anyhow::bail!("Received wrong message: {value}")
        }
    }
}