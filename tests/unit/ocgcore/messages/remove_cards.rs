use super::*;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{CardLocationKind, CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{LOCATION_GRAVE, LOCATION_MZONE, LOCATION_OVERLAY, MSG_REMOVE_CARDS};

fn sample() -> Vec<u8> {
    let mut bytes = 25u32.to_le_bytes().to_vec();
    bytes.push(MSG_REMOVE_CARDS as u8);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend([0, LOCATION_GRAVE as u8]);
    bytes.extend(256u32.to_le_bytes());
    bytes.extend(CardPositionFlags::FACE_UP_ATTACK.bits().to_le_bytes());
    bytes.extend([1, (LOCATION_MZONE | LOCATION_OVERLAY) as u8]);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend(300u32.to_le_bytes());
    bytes
}

#[test]
fn decodes_empty_and_multiple_locations() {
    let empty = [5, 0, 0, 0, MSG_REMOVE_CARDS as u8, 0, 0, 0, 0];
    assert_eq!(
        CoreMessage::try_from(empty.as_slice()).unwrap(),
        CoreMessage::RemoveCards(RemoveCardsMessageData { cards: vec![] })
    );
    let mut bytes = sample();
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::RemoveCards(RemoveCardsMessageData {
            cards: vec![
                CardLocationInfo {
                    controller: Some(Player::Local),
                    location: CardLocationKind::Known(CardLocation::Graveyard),
                    sequence: 256,
                    position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
                },
                CardLocationInfo {
                    controller: Some(Player::Opponent),
                    location: CardLocationKind::Overlay {
                        host: Some(CardLocation::MonsterZone),
                    },
                    sequence: 2,
                    position: CardLocationPosition::OverlaySequence(300),
                },
            ],
        })
    );
    bytes.extend([1, 0, 0, 0, 74]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [CoreMessage::RemoveCards(_), CoreMessage::ChainEnd]
    ));
}

#[test]
fn rejects_truncated_and_invalid_removal_messages() {
    let sample = sample();
    for length in 0..sample.len() {
        assert!(CoreMessage::try_from(&sample[..length]).is_err());
    }
    for length in [0u32, 24, 26, u32::MAX] {
        let mut bytes = sample.clone();
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for count in [0u32, 1, 3, u32::MAX] {
        let mut bytes = sample.clone();
        bytes[5..9].copy_from_slice(&count.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [9, 19] {
        let mut bytes = sample.clone();
        bytes[offset] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [10, 20] {
        let mut bytes = sample.clone();
        bytes[offset] = 0xff;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
}
