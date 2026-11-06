use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::{CardLocationInfo, CardLocationKind, read_controller};
use crate::ocgcore::utility::{read_u8, read_u32, read_u64};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainTriggerLocation {
    pub controller: Option<Player>,
    pub location: CardLocationKind,
    pub sequence: u32,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainingMessageData {
    pub card_code: u32,
    pub location: CardLocationInfo,
    pub triggering_location: ChainTriggerLocation,
    pub description: u64,
    pub chain_link: u32,
}

impl TryFrom<&[u8]> for ChainingMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 33, "Invalid chaining message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let card_code = read_u32(bytes, &mut cursor, "card code")?;
        let location = CardLocationInfo::read(bytes, &mut cursor)?;
        let controller = read_controller(bytes, &mut cursor, "triggering controller")?;
        let location_byte = read_u8(bytes, &mut cursor, "triggering location")?;
        let triggering_location = ChainTriggerLocation {
            controller,
            location: CardLocationKind::from_wire(location_byte)?,
            sequence: read_u32(bytes, &mut cursor, "triggering sequence")?,
        };
        let description = read_u64(bytes, &mut cursor, "effect description")?;
        let chain_link = read_u32(bytes, &mut cursor, "chain link")?;
        Ok(Self {
            card_code,
            location,
            triggering_location,
            description,
            chain_link,
        })
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainLinkMessageData {
    pub chain_link: u8,
}

impl TryFrom<&[u8]> for ChainLinkMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 2, "Invalid chain link message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let chain_link = read_u8(bytes, &mut cursor, "chain link")?;
        Ok(Self { chain_link })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/chaining.rs"]
mod tests;
