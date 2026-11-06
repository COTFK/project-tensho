use super::*;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{CardLocationKind, CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{LOCATION_MZONE, MSG_SWAP};

fn sample() -> Vec<u8> {
    let mut bytes = 29u32.to_le_bytes().to_vec();
    bytes.push(MSG_SWAP as u8);
    bytes.extend(100u32.to_le_bytes());
    bytes.extend([0, LOCATION_MZONE as u8]);
    bytes.extend(256u32.to_le_bytes());
    bytes.extend(CardPositionFlags::FACE_UP_DEFENSE.bits().to_le_bytes());
    bytes.extend(200u32.to_le_bytes());
    bytes.extend([1, LOCATION_MZONE as u8]);
    bytes.extend(3u32.to_le_bytes());
    bytes.extend(CardPositionFlags::FACE_DOWN_DEFENSE.bits().to_le_bytes());
    bytes
}

#[test]
fn decodes_swap_card_codes_and_source_locations() {
    let bytes = sample();
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::Swap(SwapMessageData {
            first_code: 100,
            first_location: CardLocationInfo {
                controller: Some(Player::Player),
                location: CardLocationKind::Known(CardLocation::MonsterZone),
                sequence: 256,
                position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_DEFENSE),
            },
            second_code: 200,
            second_location: CardLocationInfo {
                controller: Some(Player::Opponent),
                location: CardLocationKind::Known(CardLocation::MonsterZone),
                sequence: 3,
                position: CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE),
            },
        })
    );
    let mut framed = bytes;
    framed.extend([3, 0, 0, 0, 41, 1, 0]);
    assert!(matches!(
        parse_messages(&framed).unwrap().as_slice(),
        [CoreMessage::Swap(_), CoreMessage::NewPhase(_)]
    ));
}

#[test]
fn rejects_truncated_and_invalid_swap_messages() {
    let sample = sample();
    for length in 0..sample.len() {
        assert!(CoreMessage::try_from(&sample[..length]).is_err());
    }
    for length in [0u32, 28, 30, u32::MAX] {
        let mut bytes = sample.clone();
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [9, 23] {
        let mut bytes = sample.clone();
        bytes[offset] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [10, 24] {
        let mut bytes = sample.clone();
        bytes[offset] = 0xff;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
}
