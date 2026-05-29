use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
pub enum BattlePosition {
    FaceUpAttack = 1,
    FaceDownAttack = 2,
    FaceUpDefense = 4,
    FaceDownDefense = 8,
    FaceDown = 10,
}

impl TryFrom<u8> for BattlePosition {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(BattlePosition::FaceUpAttack),
            2 => Ok(BattlePosition::FaceDownAttack),
            4 => Ok(BattlePosition::FaceUpDefense),
            8 => Ok(BattlePosition::FaceDownDefense),
            10 => Ok(BattlePosition::FaceDown),
            _ => anyhow::bail!("Received wrong battle position value: {value}"),
        }
    }
}

impl TryFrom<u32> for BattlePosition {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(BattlePosition::FaceUpAttack),
            2 => Ok(BattlePosition::FaceDownAttack),
            4 => Ok(BattlePosition::FaceUpDefense),
            8 => Ok(BattlePosition::FaceDownDefense),
            10 => Ok(BattlePosition::FaceDown),
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
            Self::FaceDown => String::from("Face-down"),
        };
        write!(f, "{}", text)
    }
}
