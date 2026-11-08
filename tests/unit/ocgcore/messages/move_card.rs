use super::*;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{CardLocationKind, CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{LOCATION_MZONE, LOCATION_OVERLAY, PLAYER_NONE};

const SAMPLE: [u8; 33] = [
    29, 0, 0, 0, 50, 136, 86, 166, 2, 0, 2, 3, 0, 0, 0, 10, 0, 0, 0, 0, 4, 2, 0, 0, 0, 1, 0, 0, 0,
    0, 4, 0, 2,
];

#[test]
fn parses_sample_move() {
    assert_eq!(
        CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
        CoreMessage::Move(MoveMessageData {
            card_code: 44_455_560,
            previous: CardLocationInfo {
                controller: Some(Player::Local),
                location: CardLocationKind::Known(CardLocation::Hand),
                sequence: 3,
                position: CardLocationPosition::Position(
                    CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE,
                ),
            },
            current: CardLocationInfo {
                controller: Some(Player::Local),
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
