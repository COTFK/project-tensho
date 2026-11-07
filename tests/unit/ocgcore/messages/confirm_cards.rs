use super::*;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{
    LOCATION_MZONE, LOCATION_OVERLAY, MSG_CONFIRM_DECKTOP, MSG_CONFIRM_EXTRATOP, PLAYER_NONE,
};

const SAMPLE: [u8; 20] = [
    16, 0, 0, 0, 31, 1, 1, 0, 0, 0, 122, 125, 228, 3, 0, 2, 4, 0, 0, 0,
];

#[test]
fn parses_sample_confirm_cards() {
    assert_eq!(
        CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
        CoreMessage::ConfirmCards(ConfirmCardsMessageData {
            player: Player::Opponent,
            cards: vec![ConfirmedCard {
                card_code: 65_305_978,
                controller: Some(Player::Player),
                location: CardLocationKind::Known(CardLocation::Hand),
                sequence: 4,
            }],
        })
    );
}

#[test]
fn parses_deck_and_extra_top_confirmation_formats() {
    let mut bytes = SAMPLE;
    for (kind, expected) in [(MSG_CONFIRM_DECKTOP, false), (MSG_CONFIRM_EXTRATOP, true)] {
        bytes[4] = kind as u8;
        let message = CoreMessage::try_from(bytes.as_slice()).unwrap();
        assert!(match message {
            CoreMessage::ConfirmDeckTop(data) if !expected => data.cards.len() == 1,
            CoreMessage::ConfirmExtraTop(data) if expected => data.cards.len() == 1,
            _ => false,
        });
        for length in 0..bytes.len() {
            assert!(CoreMessage::try_from(&bytes[..length]).is_err());
        }
    }
}

#[test]
fn parses_empty_and_multiple_cards_with_full_width_values() {
    let empty = [6, 0, 0, 0, 31, 0, 0, 0, 0, 0];
    assert_eq!(
        CoreMessage::try_from(empty.as_slice()).unwrap(),
        CoreMessage::ConfirmCards(ConfirmCardsMessageData {
            player: Player::Player,
            cards: vec![],
        })
    );
    let mut bytes = SAMPLE.to_vec();
    bytes[..4].copy_from_slice(&26u32.to_le_bytes());
    bytes[6..10].copy_from_slice(&2u32.to_le_bytes());
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.extend([PLAYER_NONE as u8, (LOCATION_MZONE | LOCATION_OVERLAY) as u8]);
    bytes.extend(0x1234_5678u32.to_le_bytes());
    let message = ConfirmCardsMessageData::try_from(bytes.as_slice()).unwrap();
    assert_eq!(message.cards.len(), 2);
    assert_eq!(
        message.cards[1],
        ConfirmedCard {
            card_code: u32::MAX,
            controller: None,
            location: CardLocationKind::Overlay {
                host: Some(CardLocation::MonsterZone),
            },
            sequence: 0x1234_5678,
        }
    );
}

#[test]
fn rejects_truncated_frames_and_invalid_data() {
    for length in 0..SAMPLE.len() {
        assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
        assert!(parse_messages(&SAMPLE[..length]).is_err() || length == 0);
    }
    for length in [0u32, 6, 15, 17, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        assert!(parse_messages(&bytes).is_err());
    }
    for count in [0u32, 2, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[6..10].copy_from_slice(&count.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [5, 14] {
        let mut bytes = SAMPLE;
        bytes[offset] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for location in [0x0c, 0x8c, 0xff] {
        let mut bytes = SAMPLE;
        bytes[15] = location;
        let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
        assert!(format!("{error:#}").contains(&format!("{location:#04x}")));
    }
}

#[test]
fn parses_confirm_cards_among_framed_messages() {
    let mut bytes = vec![3, 0, 0, 0, 41, 1, 0];
    bytes.extend(SAMPLE);
    bytes.extend([1, 0, 0, 0, 63]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [
            CoreMessage::NewPhase(_),
            CoreMessage::ConfirmCards(ConfirmCardsMessageData { player: Player::Opponent, cards }),
            CoreMessage::SpecialSummoned
        ] if cards.len() == 1
    ));
}
