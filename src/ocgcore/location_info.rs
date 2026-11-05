use anyhow::Context;
use bitflags::bitflags;
use ocgcore_ffi::constants::{
    LOCATION_OVERLAY, PLAYER_NONE, POS_FACEDOWN_ATTACK, POS_FACEDOWN_DEFENSE, POS_FACEUP_ATTACK,
    POS_FACEUP_DEFENSE,
};

use super::constants::{CardLocation, Player};
use super::utility::{read_u8, read_u32};

bitflags! {
    #[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct CardPositionFlags: u32 {
        const FACE_UP_ATTACK = POS_FACEUP_ATTACK;
        const FACE_DOWN_ATTACK = POS_FACEDOWN_ATTACK;
        const FACE_UP_DEFENSE = POS_FACEUP_DEFENSE;
        const FACE_DOWN_DEFENSE = POS_FACEDOWN_DEFENSE;
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardLocationKind {
    None,
    Known(CardLocation),
    Overlay { host: Option<CardLocation> },
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardLocationPosition {
    Position(CardPositionFlags),
    OverlaySequence(u32),
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CardLocationInfo {
    pub controller: Option<Player>,
    pub location: CardLocationKind,
    pub sequence: u32,
    pub position: CardLocationPosition,
}

impl CardLocationInfo {
    pub(super) fn read(bytes: &[u8], cursor: &mut usize) -> anyhow::Result<Self> {
        let controller = read_u8(bytes, cursor, "card controller")?;
        let controller = if u32::from(controller) == PLAYER_NONE {
            None
        } else {
            Some(Player::try_from(controller)?)
        };
        let raw_location = read_u8(bytes, cursor, "card location")?;
        let sequence = read_u32(bytes, cursor, "card sequence")?;
        let raw_position = read_u32(bytes, cursor, "card position")?;
        let overlay = raw_location & LOCATION_OVERLAY as u8 != 0;
        let base_location = raw_location & !(LOCATION_OVERLAY as u8);
        let location = match (overlay, base_location) {
            (false, 0) => CardLocationKind::None,
            (true, 0) => CardLocationKind::Overlay { host: None },
            (true, base) => {
                CardLocationKind::Overlay {
                    host: Some(CardLocation::try_from(base).with_context(|| {
                        format!("Invalid card location byte: {raw_location:#04x}")
                    })?),
                }
            }
            (false, base) => CardLocationKind::Known(
                CardLocation::try_from(base)
                    .with_context(|| format!("Invalid card location byte: {raw_location:#04x}"))?,
            ),
        };
        let position = if overlay {
            CardLocationPosition::OverlaySequence(raw_position)
        } else {
            CardLocationPosition::Position(CardPositionFlags::from_bits_retain(raw_position))
        };
        Ok(Self {
            controller,
            location,
            sequence,
            position,
        })
    }
}
