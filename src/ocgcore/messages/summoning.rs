use crate::ocgcore::location_info::CardLocationInfo;

use super::card_event::read_card_event;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SummoningMessageData {
    pub card_code: u32,
    pub location: CardLocationInfo,
}

impl TryFrom<&[u8]> for SummoningMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let (card_code, location) = read_card_event(bytes, "summoning")?;
        Ok(Self {
            card_code,
            location,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/summoning.rs"]
mod tests;
