use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32, read_u64};
use ocgcore_ffi::constants::{
    CHINT_ATTRIBUTE, CHINT_CARD, CHINT_DESC_ADD, CHINT_DESC_REMOVE, CHINT_NUMBER, CHINT_RACE,
    CHINT_TURN,
};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardHint {
    Turn(u64),
    Card(u64),
    Race(u64),
    Attribute(u64),
    Number(u64),
    DescriptionAdd(u64),
    DescriptionRemove(u64),
    Unknown { kind: u8, value: u64 },
}

impl CardHint {
    fn from_wire(kind: u8, value: u64) -> Self {
        match u32::from(kind) {
            CHINT_TURN => Self::Turn(value),
            CHINT_CARD => Self::Card(value),
            CHINT_RACE => Self::Race(value),
            CHINT_ATTRIBUTE => Self::Attribute(value),
            CHINT_NUMBER => Self::Number(value),
            CHINT_DESC_ADD => Self::DescriptionAdd(value),
            CHINT_DESC_REMOVE => Self::DescriptionRemove(value),
            _ => Self::Unknown { kind, value },
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CardHintMessageData {
    pub location: CardLocationInfo,
    pub hint: CardHint,
}

impl TryFrom<&[u8]> for CardHintMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 20, "Invalid card hint message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let location = CardLocationInfo::read(bytes, &mut cursor)?;
        let kind = read_u8(bytes, &mut cursor, "card hint type")?;
        let value = read_u64(bytes, &mut cursor, "card hint value")?;
        Ok(Self {
            location,
            hint: CardHint::from_wire(kind, value),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/card_hint.rs"]
mod tests;
