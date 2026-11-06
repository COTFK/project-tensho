use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::{CardLocationKind, read_controller};
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmedCard {
    pub card_code: u32,
    pub controller: Option<Player>,
    pub location: CardLocationKind,
    pub sequence: u32,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ConfirmCardsMessageData {
    pub player: Player,
    pub cards: Vec<ConfirmedCard>,
}

impl TryFrom<&[u8]> for ConfirmCardsMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "confirm player")?)?;
        let count = read_u32(bytes, &mut cursor, "confirmed card count")?;
        let expected_length = count
            .checked_mul(10)
            .and_then(|size| size.checked_add(6))
            .context("Confirmed card count exceeds message size")?;
        anyhow::ensure!(
            length == expected_length,
            "Invalid confirm cards message length: {length}, expected {expected_length}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Confirm cards message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated confirm cards message");

        let mut cards = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let card_code = read_u32(bytes, &mut cursor, "confirmed card code")?;
            let controller = read_controller(bytes, &mut cursor, "confirmed card controller")?;
            let raw_location = read_u8(bytes, &mut cursor, "confirmed card location")?;
            let location = CardLocationKind::from_wire(raw_location)?;
            let sequence = read_u32(bytes, &mut cursor, "confirmed card sequence")?;
            cards.push(ConfirmedCard {
                card_code,
                controller,
                location,
                sequence,
            });
        }
        Ok(Self { player, cards })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/confirm_cards.rs"]
mod tests;
