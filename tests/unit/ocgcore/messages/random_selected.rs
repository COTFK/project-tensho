use super::*;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{CardLocationKind, CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};

const SAMPLE: [u8; 70] = [
    66, 0, 0, 0, 81, 0, 6, 0, 0, 0, //
    0, 64, 0, 0, 0, 0, 10, 0, 0, 0, //
    0, 64, 5, 0, 0, 0, 10, 0, 0, 0, //
    0, 64, 10, 0, 0, 0, 10, 0, 0, 0, //
    0, 64, 12, 0, 0, 0, 10, 0, 0, 0, //
    0, 64, 13, 0, 0, 0, 10, 0, 0, 0, //
    0, 64, 14, 0, 0, 0, 10, 0, 0, 0,
];

#[test]
fn parses_sample_random_selection() {
    let face_down = CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE;
    let cards = [0, 5, 10, 12, 13, 14].map(|sequence| CardLocationInfo {
        controller: Some(Player::Player),
        location: CardLocationKind::Known(CardLocation::ExtraDeck),
        sequence,
        position: CardLocationPosition::Position(face_down),
    });
    assert_eq!(
        CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
        CoreMessage::RandomSelected(RandomSelectedMessageData {
            player: Player::Player,
            cards: cards.to_vec(),
        })
    );
}

#[test]
fn parses_empty_random_selection() {
    assert_eq!(
        CoreMessage::try_from([6, 0, 0, 0, 81, 1, 0, 0, 0, 0].as_slice()).unwrap(),
        CoreMessage::RandomSelected(RandomSelectedMessageData {
            player: Player::Opponent,
            cards: vec![],
        })
    );
}

#[test]
fn rejects_truncated_messages_invalid_lengths_counts_and_locations() {
    for length in 0..SAMPLE.len() {
        assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
    }
    for length in [0u32, 5, 65, 67, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for count in [0u32, 5, 7, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[6..10].copy_from_slice(&count.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    let mut bytes = SAMPLE;
    bytes[5] = 2;
    assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    for location in [0x0c, 0x8c, 0xff] {
        let mut bytes = SAMPLE;
        bytes[11] = location;
        let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
        assert!(format!("{error:#}").contains(&format!("{location:#04x}")));
    }
}

#[test]
fn parses_random_selection_among_framed_messages() {
    let mut bytes = SAMPLE.to_vec();
    bytes.extend([1, 0, 0, 0, 74]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [
            CoreMessage::RandomSelected(RandomSelectedMessageData { cards, .. }),
            CoreMessage::ChainEnd
        ] if cards.len() == 6
    ));
}
