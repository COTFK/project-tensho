use crate::ocgcore::location_info::CardLocationInfo;

use super::card_locations::read_card_locations;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct RemoveCardsMessageData {
    pub cards: Vec<CardLocationInfo>,
}

impl TryFrom<&[u8]> for RemoveCardsMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            cards: read_card_locations(bytes, "remove cards")?,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/remove_cards.rs"]
mod tests;
