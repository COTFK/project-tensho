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
        sequence: u8,
    },
    NormalSummon {
        sequence: u8,
    },
    SelectCard {
        sequence: u8,
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
            Self::Chain { sequence } => vec![*sequence, 0, 0, 0],
            Self::NormalSummon { sequence } => vec![0, 0, *sequence, 0],
            Self::SelectCard { sequence } => vec![2, 0, 0, 0, 1, 0, 0, 0, *sequence],
        }
    }
}
