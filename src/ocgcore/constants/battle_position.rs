use std::fmt::Display;

use ocgcore_ffi::constants::{
    POS_FACEDOWN, POS_FACEDOWN_ATTACK, POS_FACEDOWN_DEFENSE, POS_FACEUP_ATTACK, POS_FACEUP_DEFENSE,
};

#[derive(Hash, Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
pub enum BattlePosition {
    FaceUpAttack = POS_FACEUP_ATTACK as isize,
    FaceDownAttack = POS_FACEDOWN_ATTACK as isize,
    FaceUpDefense = POS_FACEUP_DEFENSE as isize,
    FaceDownDefense = POS_FACEDOWN_DEFENSE as isize,
    FaceDown = POS_FACEDOWN as isize,
}

impl TryFrom<u8> for BattlePosition {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        Self::try_from(u32::from(value))
    }
}

impl TryFrom<u32> for BattlePosition {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> anyhow::Result<Self, Self::Error> {
        match value {
            POS_FACEUP_ATTACK => Ok(BattlePosition::FaceUpAttack),
            POS_FACEDOWN_ATTACK => Ok(BattlePosition::FaceDownAttack),
            POS_FACEUP_DEFENSE => Ok(BattlePosition::FaceUpDefense),
            POS_FACEDOWN_DEFENSE => Ok(BattlePosition::FaceDownDefense),
            POS_FACEDOWN => Ok(BattlePosition::FaceDown),
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
