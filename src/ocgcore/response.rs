use super::constants::BattlePosition;

#[derive(Hash, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Response {
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
    SetMonster {
        index: u8,
    },
    SetSpellTrap {
        index: u8,
    },
    SelectCard {
        indices: Vec<u8>,
    },
    SortCard {
        indices: Vec<u8>,
    },
    SelectUnselectCard {
        index: u32,
    },
    SelectPosition {
        position: BattlePosition,
    },
    SelectTributes {
        tributes: Vec<u8>,
    },
    SelectOption {
        index: u8,
    },
}

impl Response {
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
            Self::SelectOption { index } => vec![*index, 0, 0, 0],
            Self::NormalSummon { index } => vec![0, 0, *index, 0],
            Self::SpecialSummon { index } => vec![1, 0, *index, 0],
            Self::SetMonster { index } => vec![3, 0, *index, 0],
            Self::SetSpellTrap { index } => vec![4, 0, *index, 0],
            Self::Activate { index } => vec![5, 0, *index, 0],
            Self::SelectCard { indices } => {
                let mut response = vec![2, 0, 0, 0];
                response.extend((indices.len() as u32).to_le_bytes());
                response.extend(indices.iter().copied());

                response
            }
            Self::SortCard { indices } => {
                let mut response = Vec::new();
                response.extend(indices.iter().copied());

                response
            }
            Self::SelectUnselectCard { index } => {
                let index_bytes = index.to_le_bytes();
                vec![
                    1,
                    0,
                    0,
                    0,
                    index_bytes[0],
                    index_bytes[1],
                    index_bytes[2],
                    index_bytes[3],
                ]
            }
            Self::SelectPosition { position } => vec![*position as u8, 0, 0, 0],
            Self::SelectTributes { tributes } => {
                let mut response = vec![2, 0, 0, 0];
                response.extend((tributes.len() as u32).to_le_bytes());
                response.extend(tributes.iter().copied());

                response
            }
        }
    }
}
