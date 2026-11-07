use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::location_info::{CardLocationInfo, CardLocationKind};
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShuffleSetCardMessageData {
    pub location: CardLocation,
    pub previous: Vec<CardLocationInfo>,
    pub current: Vec<Option<CardLocationInfo>>,
}

impl TryFrom<&[u8]> for ShuffleSetCardMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let location = CardLocation::try_from(read_u8(bytes, &mut cursor, "shuffle location")?)?;
        anyhow::ensure!(
            matches!(
                location,
                CardLocation::MonsterZone | CardLocation::SpellTrapZone
            ),
            "Invalid set-card shuffle location"
        );
        let count = read_u8(bytes, &mut cursor, "shuffled card count")?;
        let expected = 3 + u32::from(count) * 20;
        anyhow::ensure!(
            length == expected,
            "Invalid set-card shuffle message length: {length}, expected {expected}"
        );
        let end = usize::try_from(length)? + 4;
        anyhow::ensure!(bytes.len() >= end, "Truncated set-card shuffle message");

        let mut previous = Vec::with_capacity(count as usize);
        for _ in 0..count {
            previous.push(CardLocationInfo::read(bytes, &mut cursor)?);
        }
        let mut current = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let info = CardLocationInfo::read(bytes, &mut cursor)?;
            current.push((info.location != CardLocationKind::None).then_some(info));
        }
        Ok(Self {
            location,
            previous,
            current,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/shuffle_set_card.rs"]
mod tests;
