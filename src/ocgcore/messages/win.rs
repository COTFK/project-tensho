use crate::ocgcore::constants::WinReason;
use crate::ocgcore::constants::Winner;
use crate::ocgcore::utility::read_u8;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WinMessageData {
    pub winner: Winner,
    pub reason: WinReason,
}

impl TryFrom<&[u8]> for WinMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let mut cursor = 5;
        let winner = Winner::try_from(read_u8(bytes, &mut cursor, "winner")?)?;
        let reason = WinReason::from(read_u8(bytes, &mut cursor, "win reason")?);

        Ok(Self { winner, reason })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/win.rs"]
mod tests;
