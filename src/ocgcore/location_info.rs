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

impl CardLocationKind {
    pub(super) fn from_wire(raw_location: u8) -> anyhow::Result<Self> {
        let overlay = raw_location & LOCATION_OVERLAY as u8 != 0;
        let base_location = raw_location & !(LOCATION_OVERLAY as u8);
        Ok(match (overlay, base_location) {
            (false, 0) => Self::None,
            (true, 0) => Self::Overlay { host: None },
            (true, base) => {
                Self::Overlay {
                    host: Some(CardLocation::try_from(base).with_context(|| {
                        format!("Invalid card location byte: {raw_location:#04x}")
                    })?),
                }
            }
            (false, base) => Self::Known(
                CardLocation::try_from(base)
                    .with_context(|| format!("Invalid card location byte: {raw_location:#04x}"))?,
            ),
        })
    }
}

pub(super) fn read_controller(
    bytes: &[u8],
    cursor: &mut usize,
    label: &str,
) -> anyhow::Result<Option<Player>> {
    let controller = read_u8(bytes, cursor, label)?;
    if u32::from(controller) == PLAYER_NONE {
        Ok(None)
    } else {
        Ok(Some(Player::try_from(controller)?))
    }
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
        let controller = read_controller(bytes, cursor, "card controller")?;
        let raw_location = read_u8(bytes, cursor, "card location")?;
        let sequence = read_u32(bytes, cursor, "card sequence")?;
        let raw_position = read_u32(bytes, cursor, "card position")?;
        let overlay = raw_location & LOCATION_OVERLAY as u8 != 0;
        let location = CardLocationKind::from_wire(raw_location)?;
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
