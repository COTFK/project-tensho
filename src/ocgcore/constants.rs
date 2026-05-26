use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardLocation {
    Deck = 1,
    Hand = 2,
    MonsterZone = 4,
    SpellTrapZone = 8,
    Graveyard = 16,
    ExtraDeck = 64,
}

impl TryFrom<u8> for CardLocation {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(CardLocation::Deck),
            2 => Ok(CardLocation::Hand),
            4 => Ok(CardLocation::MonsterZone),
            8 => Ok(CardLocation::SpellTrapZone),
            16 => Ok(CardLocation::Graveyard),
            64 => Ok(CardLocation::ExtraDeck),
            _ => anyhow::bail!("Received wrong location value: {value}"),
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
            _ => anyhow::bail!("Received wrong owner value"),
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
            _ => anyhow::bail!("Received wrong controller value"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
pub enum BattlePosition {
    FaceUpAttack = 1,
    FaceDownAttack = 2,
    FaceUpDefense = 4,
    FaceDownDefense = 8,
}

impl TryFrom<u8> for BattlePosition {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(BattlePosition::FaceUpAttack),
            2 => Ok(BattlePosition::FaceDownAttack),
            4 => Ok(BattlePosition::FaceUpDefense),
            8 => Ok(BattlePosition::FaceDownDefense),
            _ => anyhow::bail!("Received wrong battle position value: {value}"),
        }
    }
}

impl Display for BattlePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::FaceUpAttack => String::from("Face-up Attack Position"),
            Self::FaceDownAttack => String::from("Face-down Attack Position"),
            Self::FaceUpDefense => String::from("Face-up Defense Position"),
            Self::FaceDownDefense => String::from("Face-down Defense Position"),
        };
        write!(f, "{}", text)
    }
}
