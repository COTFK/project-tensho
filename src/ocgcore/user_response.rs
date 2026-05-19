#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UserResponse {
    ActivatePromptedEffect,
    RefusePromptedEffect,
    PassPriority,
    Place { controller: u8, location: u8, index: u8 },
    Chain { sequence: u8 },
    NormalSummon { sequence: u8 }
}

impl UserResponse {
    pub fn get_response_bytes(&self) -> Vec<u8> {
        let response = match self {
            Self::ActivatePromptedEffect => vec![1, 0, 0, 0],
            Self::RefusePromptedEffect => vec![0, 0, 0, 0],
            Self::PassPriority => vec![255, 255, 255, 255],
            Self::Place {
                controller,
                location,
                index
            } => vec![*controller, *location, *index],
            Self::Chain { sequence } => vec![*sequence, 0, 0, 0],
            Self::NormalSummon { sequence } => vec![0, 0, *sequence, 0]
        };

        return response;
    }
}
