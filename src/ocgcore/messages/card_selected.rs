use crate::ocgcore::location_info::CardLocationInfo;

use super::card_locations::read_card_locations;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct CardSelectedMessageData {
    pub cards: Vec<CardLocationInfo>,
}

impl TryFrom<&[u8]> for CardSelectedMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            cards: read_card_locations(bytes, "card selected")?,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/card_selected.rs"]
mod tests;
