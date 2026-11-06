use bitflags::bitflags;
use ocgcore_ffi::constants::{
    REASON_ADJUST, REASON_BATTLE, REASON_COST, REASON_DESTROY, REASON_DISCARD, REASON_DISSUMMON,
    REASON_DRAW, REASON_EFFECT, REASON_FLIP, REASON_FUSION, REASON_LINK, REASON_LOST_TARGET,
    REASON_MATERIAL, REASON_RDAMAGE, REASON_REDIRECT, REASON_RELEASE, REASON_REPLACE,
    REASON_RETURN, REASON_RITUAL, REASON_RRECOVER, REASON_RULE, REASON_SPSUMMON, REASON_SUMMON,
    REASON_SYNCHRO, REASON_TEMPORARY, REASON_XYZ,
};

use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

bitflags! {
    #[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct MoveReason: u32 {
        const DESTROY = REASON_DESTROY;
        const RELEASE = REASON_RELEASE;
        const TEMPORARY = REASON_TEMPORARY;
        const MATERIAL = REASON_MATERIAL;
        const SUMMON = REASON_SUMMON;
        const BATTLE = REASON_BATTLE;
        const EFFECT = REASON_EFFECT;
        const COST = REASON_COST;
        const ADJUST = REASON_ADJUST;
        const LOST_TARGET = REASON_LOST_TARGET;
        const RULE = REASON_RULE;
        const SPECIAL_SUMMON = REASON_SPSUMMON;
        const DISABLED_SUMMON = REASON_DISSUMMON;
        const FLIP = REASON_FLIP;
        const DISCARD = REASON_DISCARD;
        const DAMAGE = REASON_RDAMAGE;
        const RECOVER = REASON_RRECOVER;
        const RETURN = REASON_RETURN;
        const FUSION = REASON_FUSION;
        const SYNCHRO = REASON_SYNCHRO;
        const RITUAL = REASON_RITUAL;
        const XYZ = REASON_XYZ;
        const REPLACE = REASON_REPLACE;
        const DRAW = REASON_DRAW;
        const REDIRECT = REASON_REDIRECT;
        const LINK = REASON_LINK;
    }

}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MoveMessageData {
    pub card_code: u32,
    pub previous: CardLocationInfo,
    pub current: CardLocationInfo,
    pub reason: MoveReason,
}

impl TryFrom<&[u8]> for MoveMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 29, "Invalid move message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let card_code = read_u32(bytes, &mut cursor, "card code")?;
        let previous = CardLocationInfo::read(bytes, &mut cursor)?;
        let current = CardLocationInfo::read(bytes, &mut cursor)?;
        let reason = MoveReason::from_bits_retain(read_u32(bytes, &mut cursor, "move reason")?);
        Ok(Self {
            card_code,
            previous,
            current,
            reason,
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/move_card.rs"]
mod tests;
