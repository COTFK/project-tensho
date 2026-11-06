use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SwapMessageData {
    pub first_code: u32,
    pub first_location: CardLocationInfo,
    pub second_code: u32,
    pub second_location: CardLocationInfo,
}

impl TryFrom<&[u8]> for SwapMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 29, "Invalid swap message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        Ok(Self {
            first_code: read_u32(bytes, &mut cursor, "first card code")?,
            first_location: CardLocationInfo::read(bytes, &mut cursor)?,
            second_code: read_u32(bytes, &mut cursor, "second card code")?,
            second_location: CardLocationInfo::read(bytes, &mut cursor)?,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/swap.rs"]
mod tests;
