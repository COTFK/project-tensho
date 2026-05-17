#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DuelStatus {
    End,
    Awaiting,
    Continue
}

impl TryFrom<u32> for DuelStatus {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> anyhow::Result<Self, Self::Error> {
        match value {
            0 => Ok(DuelStatus::End),
            1 => Ok(DuelStatus::Awaiting),
            2 => Ok(DuelStatus::Continue),
            _ => anyhow::bail!("Received invalid duel status")
        }
    }
}