use super::*;
use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::{CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{LOCATION_MZONE, LOCATION_SZONE, MSG_SHUFFLE_SET_CARD};

fn sample() -> Vec<u8> {
    let mut bytes = 43u32.to_le_bytes().to_vec();
    bytes.extend([MSG_SHUFFLE_SET_CARD as u8, LOCATION_MZONE as u8, 2]);
    bytes.extend([0, LOCATION_MZONE as u8]);
    bytes.extend(256u32.to_le_bytes());
    bytes.extend(CardPositionFlags::FACE_DOWN_DEFENSE.bits().to_le_bytes());
    bytes.extend([0, LOCATION_MZONE as u8]);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend(CardPositionFlags::FACE_DOWN_DEFENSE.bits().to_le_bytes());
    bytes.extend([0, LOCATION_MZONE as u8]);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend(CardPositionFlags::FACE_DOWN_DEFENSE.bits().to_le_bytes());
    bytes.extend([0; 10]);
    bytes
}

#[test]
fn decodes_set_shuffle_with_revealed_and_concealed_destinations() {
    let bytes = sample();
    let first = CardLocationInfo {
        controller: Some(Player::Local),
        location: CardLocationKind::Known(CardLocation::MonsterZone),
        sequence: 256,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE),
    };
    let second = CardLocationInfo {
        sequence: 2,
        ..first
    };
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::ShuffleSetCard(ShuffleSetCardMessageData {
            location: CardLocation::MonsterZone,
            previous: vec![first, second],
            current: vec![Some(second), None],
        })
    );
    let mut framed = bytes;
    framed.extend([1, 0, 0, 0, 74]);
    assert!(matches!(
        parse_messages(&framed).unwrap().as_slice(),
        [CoreMessage::ShuffleSetCard(_), CoreMessage::ChainEnd]
    ));
    let empty = [
        3,
        0,
        0,
        0,
        MSG_SHUFFLE_SET_CARD as u8,
        LOCATION_SZONE as u8,
        0,
    ];
    assert_eq!(
        CoreMessage::try_from(empty.as_slice()).unwrap(),
        CoreMessage::ShuffleSetCard(ShuffleSetCardMessageData {
            location: CardLocation::SpellTrapZone,
            previous: vec![],
            current: vec![],
        })
    );
}

#[test]
fn rejects_truncated_and_invalid_set_shuffle_frames() {
    let sample = sample();
    for length in 0..sample.len() {
        assert!(CoreMessage::try_from(&sample[..length]).is_err());
    }
    for length in [0u32, 3, 42, 44, u32::MAX] {
        let mut bytes = sample.clone();
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for count in [0, 1, 3, u8::MAX] {
        let mut bytes = sample.clone();
        bytes[6] = count;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for location in [0, 1, 0xff] {
        let mut bytes = sample.clone();
        bytes[5] = location;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [7, 17, 27, 37] {
        let mut bytes = sample.clone();
        bytes[offset] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [8, 18, 28, 38] {
        let mut bytes = sample.clone();
        bytes[offset] = 0xff;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
}
