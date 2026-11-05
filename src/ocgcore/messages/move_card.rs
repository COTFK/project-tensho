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
mod tests {
    use super::*;
    use crate::ocgcore::constants::{CardLocation, Player};
    use crate::ocgcore::location_info::{
        CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::{CoreMessage, parse_messages};
    use ocgcore_ffi::constants::{LOCATION_MZONE, LOCATION_OVERLAY, PLAYER_NONE};

    const SAMPLE: [u8; 33] = [
        29, 0, 0, 0, 50, 136, 86, 166, 2, 0, 2, 3, 0, 0, 0, 10, 0, 0, 0, 0, 4, 2, 0, 0, 0, 1, 0, 0,
        0, 0, 4, 0, 2,
    ];

    #[test]
    fn parses_sample_move() {
        assert_eq!(
            CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
            CoreMessage::Move(MoveMessageData {
                card_code: 44_455_560,
                previous: CardLocationInfo {
                    controller: Some(Player::Player),
                    location: CardLocationKind::Known(CardLocation::Hand),
                    sequence: 3,
                    position: CardLocationPosition::Position(
                        CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE,
                    ),
                },
                current: CardLocationInfo {
                    controller: Some(Player::Player),
                    location: CardLocationKind::Known(CardLocation::MonsterZone),
                    sequence: 2,
                    position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
                },
                reason: MoveReason::RULE | MoveReason::DRAW,
            })
        );
    }

    #[test]
    fn handles_overlay_absent_and_unknown_flags() {
        let mut bytes = SAMPLE;
        bytes[5..9].copy_from_slice(&0u32.to_le_bytes());
        bytes[9] = 1;
        bytes[10] = (LOCATION_MZONE | LOCATION_OVERLAY) as u8;
        bytes[15..19].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        bytes[19] = PLAYER_NONE as u8;
        bytes[20] = 0;
        bytes[25..29].copy_from_slice(&0x8000_0000u32.to_le_bytes());
        bytes[29..33].copy_from_slice(&0x8000_0040u32.to_le_bytes());
        let message = MoveMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(message.card_code, 0);
        assert_eq!(message.previous.controller, Some(Player::Opponent));
        assert_eq!(
            message.previous.location,
            CardLocationKind::Overlay {
                host: Some(CardLocation::MonsterZone)
            }
        );
        assert_eq!(
            message.previous.position,
            CardLocationPosition::OverlaySequence(0x1234_5678)
        );
        assert_eq!(message.current.controller, None);
        assert_eq!(message.current.location, CardLocationKind::None);
        assert_eq!(
            message.current.position,
            CardLocationPosition::Position(CardPositionFlags::from_bits_retain(0x8000_0000))
        );
        assert_eq!(message.reason.bits(), 0x8000_0040);

        bytes[10] = 0x90;
        let message = MoveMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(
            message.previous.location,
            CardLocationKind::Overlay {
                host: Some(CardLocation::Graveyard)
            }
        );
        bytes[10] = LOCATION_OVERLAY as u8;
        let message = MoveMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(
            message.previous.location,
            CardLocationKind::Overlay { host: None }
        );
        assert_eq!(
            message.previous.position,
            CardLocationPosition::OverlaySequence(0x1234_5678)
        );
    }

    #[test]
    fn rejects_truncated_and_invalid_messages() {
        for length in 0..SAMPLE.len() {
            assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
        }
        for length in [0u32, 28, 30, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        }
        let mut bytes = SAMPLE;
        bytes[9] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        bytes = SAMPLE;
        bytes[19] = 255;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        for offset in [10, 20] {
            for location in [0x0c, 0x8c, 0xff] {
                let mut bytes = SAMPLE;
                bytes[offset] = location;
                let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
                assert!(
                    format!("{error:#}").contains(&format!("{location:#04x}")),
                    "error did not include raw location: {error:#}"
                );
            }
        }
    }

    #[test]
    fn parses_multiple_framed_messages() {
        let mut bytes = SAMPLE.to_vec();
        bytes.extend([3, 0, 0, 0, 41, 1, 0]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [CoreMessage::Move(_), CoreMessage::NewPhase(_)]
        ));
    }
}
