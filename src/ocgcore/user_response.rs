use super::constants::BattlePosition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UserResponse {
    Yes,
    No,
    PassPriority,
    Place {
        controller: u8,
        location: u8,
        index: u8,
    },
    Chain {
        index: u8,
    },
    Activate {
        index: u8,
    },
    SpecialSummon {
        index: u8,
    },
    NormalSummon {
        index: u8,
    },
    SelectCard {
        index: u8,
    },
    SelectPosition {
        position: BattlePosition,
    },
}

impl UserResponse {
    pub fn get_response_bytes(&self) -> Vec<u8> {
        match self {
            Self::Yes => vec![1, 0, 0, 0],
            Self::No => vec![0, 0, 0, 0],
            Self::PassPriority => vec![255, 255, 255, 255],
            Self::Place {
                controller,
                location,
                index,
            } => vec![*controller, *location, *index],
            Self::Chain { index } => vec![*index, 0, 0, 0],
            Self::Activate { index } => vec![5, 0, *index, 0],
            Self::SpecialSummon { index } => vec![1, 0, *index, 0],
            Self::NormalSummon { index } => vec![0, 0, *index, 0],
            Self::SelectCard { index } => vec![2, 0, 0, 0, 1, 0, 0, 0, *index],
            Self::SelectPosition { position } => vec![*position as u8, 0, 0, 0],
        }
    }
}
