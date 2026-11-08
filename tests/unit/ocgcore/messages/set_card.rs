use super::*;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{CardLocationKind, CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};

const SAMPLE: [u8; 19] = [
    15, 0, 0, 0, 54, 124, 73, 119, 5, 0, 8, 3, 0, 0, 0, 10, 0, 0, 0,
];

#[test]
fn parses_sample_set_card() {
    assert_eq!(
        CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
        CoreMessage::SetCard(SetCardMessageData {
            card_code: 91_703_676,
            location: CardLocationInfo {
                controller: Some(Player::Local),
                location: CardLocationKind::Known(CardLocation::SpellTrapZone),
                sequence: 3,
                position: CardLocationPosition::Position(
                    CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE,
                ),
            },
        })
    );
}

#[test]
fn preserves_hidden_code_and_full_width_location() {
    let mut bytes = SAMPLE;
    bytes[5..9].copy_from_slice(&0u32.to_le_bytes());
    bytes[9] = 1;
    bytes[11..15].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    bytes[15..19].copy_from_slice(&0x8000_0008u32.to_le_bytes());
    let message = SetCardMessageData::try_from(bytes.as_slice()).unwrap();
    assert_eq!(message.card_code, 0);
    assert_eq!(message.location.controller, Some(Player::Opponent));
    assert_eq!(message.location.sequence, 0x1234_5678);
    assert_eq!(
        message.location.position,
        CardLocationPosition::Position(CardPositionFlags::from_bits_retain(0x8000_0008))
    );
}

#[test]
fn rejects_truncated_messages_invalid_lengths_and_locations() {
    for length in 0..SAMPLE.len() {
        assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
        assert!(parse_messages(&SAMPLE[..length]).is_err() || length == 0);
    }
    for length in [0u32, 14, 16, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        assert!(parse_messages(&bytes).is_err());
    }
    let mut bytes = SAMPLE;
    bytes[9] = 3;
    assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    for location in [0x0c, 0x8c, 0xff] {
        let mut bytes = SAMPLE;
        bytes[10] = location;
        let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
        assert!(format!("{error:#}").contains(&format!("{location:#04x}")));
    }
}

#[test]
fn parses_set_card_among_framed_messages() {
    let mut bytes = SAMPLE.to_vec();
    bytes.extend([1, 0, 0, 0, 74]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [CoreMessage::SetCard(_), CoreMessage::ChainEnd]
    ));
}
