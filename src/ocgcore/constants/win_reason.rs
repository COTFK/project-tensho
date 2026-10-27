#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WinReason {
    LifePointsDepleted,
    DeckOut,
    Custom(u8),
}

impl From<u8> for WinReason {
    fn from(reason: u8) -> Self {
        match reason {
            1 => Self::LifePointsDepleted,
            2 => Self::DeckOut,
            reason => Self::Custom(reason),
        }
    }
}
