use super::*;
use crate::ocgcore::messages::{CoreMessage, parse_messages};

const SAMPLE: [u8; 14] = [10, 0, 0, 0, 53, 42, 0, 0, 0, 1, 4, 255, 8, 1];

#[test]
fn decodes_position_change_and_preserves_flags() {
    let message = PositionChangeMessageData::try_from(SAMPLE.as_slice()).unwrap();
    assert_eq!(
        message,
        PositionChangeMessageData {
            card_code: 42,
            controller: Player::Opponent,
            location: CardLocation::MonsterZone,
            sequence: 255,
            previous_position: CardPositionFlags::FACE_DOWN_DEFENSE,
            current_position: CardPositionFlags::FACE_UP_ATTACK,
        }
    );
    let mut bytes = SAMPLE;
    bytes[5..9].copy_from_slice(&0u32.to_le_bytes());
    bytes[12] = 0x8a;
    bytes[13] = 0x85;
    let message = PositionChangeMessageData::try_from(bytes.as_slice()).unwrap();
    assert_eq!(message.card_code, 0);
    assert_eq!(message.previous_position.bits(), 0x8a);
    assert_eq!(message.current_position.bits(), 0x85);
}

#[test]
fn rejects_truncated_and_invalid_position_changes() {
    for length in 0..SAMPLE.len() {
        assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
    }
    for length in [0u32, 9, 11, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for (offset, value) in [(9, 2), (9, 255), (10, 0), (10, 0x84), (10, 255)] {
        let mut bytes = SAMPLE;
        bytes[offset] = value;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
}

#[test]
fn parses_position_changes_in_framed_order() {
    let mut bytes = SAMPLE.to_vec();
    bytes.extend([1, 0, 0, 0, 65]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [CoreMessage::PositionChange(_), CoreMessage::FlipSummoned]
    ));
}
