use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionChangeMessageData {
    pub card_code: u32,
    pub controller: Player,
    pub location: CardLocation,
    pub sequence: u32,
    pub previous_position: CardPositionFlags,
    pub current_position: CardPositionFlags,
}

impl TryFrom<&[u8]> for PositionChangeMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(
            length == 10,
            "Invalid position change message length: {length}"
        );
        read_u8(bytes, &mut cursor, "message type")?;
        let card_code = read_u32(bytes, &mut cursor, "card code")?;
        let controller = Player::try_from(read_u8(bytes, &mut cursor, "card controller")?)?;
        let location = CardLocation::try_from(read_u8(bytes, &mut cursor, "card location")?)?;
        let sequence = u32::from(read_u8(bytes, &mut cursor, "card sequence")?);
        let previous_position = CardPositionFlags::from_bits_retain(u32::from(read_u8(
            bytes,
            &mut cursor,
            "previous position",
        )?));
        let current_position = CardPositionFlags::from_bits_retain(u32::from(read_u8(
            bytes,
            &mut cursor,
            "current position",
        )?));
        Ok(Self {
            card_code,
            controller,
            location,
            sequence,
            previous_position,
            current_position,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/position_change.rs"]
mod tests;
