use crate::ocgcore::location_info::CardLocationInfo;

use super::card_locations::read_card_locations;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct BecomeTargetMessageData {
    pub targets: Vec<CardLocationInfo>,
}

impl TryFrom<&[u8]> for BecomeTargetMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            targets: read_card_locations(bytes, "become target")?,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/become_target.rs"]
mod tests;
