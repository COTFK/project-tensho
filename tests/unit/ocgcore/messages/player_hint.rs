use super::*;
use crate::ocgcore::constants::Player;
use crate::ocgcore::messages::{CoreMessage, parse_messages};

const SAMPLE: [u8; 15] = [11, 0, 0, 0, 165, 0, 6, 2, 0, 128, 19, 245, 46, 0, 0];

#[test]
fn parses_sample_player_hint() {
    assert_eq!(
        CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
        CoreMessage::PlayerHint(PlayerHintMessageData {
            player: Player::Player,
            hint: PlayerHint::DescriptionAdd(0x0000_2ef5_1380_0002),
        })
    );
}

#[test]
fn parses_description_remove_and_preserves_value() {
    let value = 0xfedc_ba98_7654_3210u64;
    let mut bytes = SAMPLE;
    bytes[6] = PHINT_DESC_REMOVE as u8;
    bytes[7..15].copy_from_slice(&value.to_le_bytes());
    assert_eq!(
        PlayerHintMessageData::try_from(bytes.as_slice())
            .unwrap()
            .hint,
        PlayerHint::DescriptionRemove(value)
    );
}

#[test]
fn preserves_unknown_hint_kinds() {
    for kind in [0, 1, 8, u8::MAX] {
        let mut bytes = SAMPLE;
        bytes[6] = kind;
        assert_eq!(
            PlayerHintMessageData::try_from(bytes.as_slice())
                .unwrap()
                .hint,
            PlayerHint::Unknown {
                kind,
                value: 0x0000_2ef5_1380_0002,
            }
        );
    }
}

#[test]
fn rejects_truncated_messages_invalid_lengths_and_players() {
    for length in 0..SAMPLE.len() {
        assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
    }
    for length in [0u32, 10, 12, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    let mut bytes = SAMPLE;
    bytes[5] = 2;
    assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
}

#[test]
fn parses_player_hint_among_framed_messages() {
    let mut bytes = SAMPLE.to_vec();
    bytes.extend([1, 0, 0, 0, 63]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [
            CoreMessage::PlayerHint(PlayerHintMessageData {
                player: Player::Player,
                hint: PlayerHint::DescriptionAdd(_),
            }),
            CoreMessage::SpecialSummoned
        ]
    ));
}
