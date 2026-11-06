use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32, read_u64};
use ocgcore_ffi::constants::{
    HINT_ATTRIB, HINT_CARD, HINT_CODE, HINT_EFFECT, HINT_EVENT, HINT_MESSAGE, HINT_NUMBER,
    HINT_OPSELECTED, HINT_RACE, HINT_SELECTMSG, HINT_ZONE,
};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hint {
    Event(u64),
    Message(u64),
    SelectMessage(u64),
    OptionSelected(u64),
    Effect(u64),
    Race(u64),
    Attribute(u64),
    Code(u64),
    Number(u64),
    Card(u64),
    Zone(u64),
    Unknown { kind: u8, value: u64 },
}

impl Hint {
    fn from_wire(kind: u8, value: u64) -> Self {
        match u32::from(kind) {
            HINT_EVENT => Self::Event(value),
            HINT_MESSAGE => Self::Message(value),
            HINT_SELECTMSG => Self::SelectMessage(value),
            HINT_OPSELECTED => Self::OptionSelected(value),
            HINT_EFFECT => Self::Effect(value),
            HINT_RACE => Self::Race(value),
            HINT_ATTRIB => Self::Attribute(value),
            HINT_CODE => Self::Code(value),
            HINT_NUMBER => Self::Number(value),
            HINT_CARD => Self::Card(value),
            HINT_ZONE => Self::Zone(value),
            _ => Self::Unknown { kind, value },
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HintMessageData {
    pub player: Player,
    pub hint: Hint,
}

impl TryFrom<&[u8]> for HintMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 11, "Invalid hint message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let kind = read_u8(bytes, &mut cursor, "hint type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "hint player")?)?;
        let value = read_u64(bytes, &mut cursor, "hint value")?;
        Ok(Self {
            player,
            hint: Hint::from_wire(kind, value),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/hint.rs"]
mod tests;
