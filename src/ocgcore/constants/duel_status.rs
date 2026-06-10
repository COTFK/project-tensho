#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DuelStatus {
    End = 0,
    Awaiting = 1,
    Continue = 2,
}

impl TryFrom<i32> for DuelStatus {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> anyhow::Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::End),
            1 => Ok(Self::Awaiting),
            2 => Ok(Self::Continue),
            _ => anyhow::bail!("Received invalid duel status"),
        }
    }
}
