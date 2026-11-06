use crate::ocgcore::constants::Phase;
use crate::ocgcore::utility::{read_u16, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NewPhaseMessageData {
    pub phase: Phase,
}

impl TryFrom<&[u8]> for NewPhaseMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 3, "Invalid new phase message length: {length}");
        cursor += 1;
        let phase = Phase::try_from(read_u16(bytes, &mut cursor, "phase")?)?;
        Ok(Self { phase })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/new_phase.rs"]
mod tests;
