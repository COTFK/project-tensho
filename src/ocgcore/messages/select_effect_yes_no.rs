use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectEffectYNMessageData {
    pub player: u8,
    pub card_code: u32,
    pub controller: CardController,
    pub location: CardLocation,
    pub sequence: u8,
}

impl TryFrom<&[u8]> for SelectEffectYNMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[5];
        let card_code = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        let controller = CardController::try_from(bytes[10])?;
        let location = CardLocation::try_from(bytes[11]).unwrap();
        let sequence = bytes[12];

        Ok(SelectEffectYNMessageData {
            player,
            card_code,
            controller,
            location,
            sequence,
        })
    }
}
