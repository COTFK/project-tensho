use super::*;
use crate::ocgcore::messages::{CoreMessage, parse_messages};

fn bytes(kind: u8, player: u8, value: u64) -> Vec<u8> {
    let mut bytes = vec![11, 0, 0, 0, 2, kind, player];
    bytes.extend(value.to_le_bytes());
    bytes
}

#[test]
fn parses_draw_phase_example() {
    let bytes = [11, 0, 0, 0, 2, 1, 0, 27, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::Hint(HintMessageData {
            player: Player::Player,
            hint: Hint::Event(27),
        })
    );
}

#[test]
fn parses_every_hint_type_without_narrowing_values() {
    let value = 0xfedc_ba98_7654_3210;
    for (kind, hint) in [
        (HINT_EVENT, Hint::Event(value)),
        (HINT_MESSAGE, Hint::Message(value)),
        (HINT_SELECTMSG, Hint::SelectMessage(value)),
        (HINT_OPSELECTED, Hint::OptionSelected(value)),
        (HINT_EFFECT, Hint::Effect(value)),
        (HINT_RACE, Hint::Race(value)),
        (HINT_ATTRIB, Hint::Attribute(value)),
        (HINT_CODE, Hint::Code(value)),
        (HINT_NUMBER, Hint::Number(value)),
        (HINT_CARD, Hint::Card(value)),
        (HINT_ZONE, Hint::Zone(value)),
    ] {
        assert_eq!(
            CoreMessage::try_from(bytes(kind as u8, 1, value).as_slice()).unwrap(),
            CoreMessage::Hint(HintMessageData {
                player: Player::Opponent,
                hint,
            })
        );
    }
}

#[test]
fn preserves_unknown_types_and_zero_values() {
    for kind in [0, 12, 255] {
        assert_eq!(
            HintMessageData::try_from(bytes(kind, 0, u64::MAX).as_slice())
                .unwrap()
                .hint,
            Hint::Unknown {
                kind,
                value: u64::MAX,
            }
        );
    }
    assert_eq!(
        HintMessageData::try_from(bytes(HINT_SELECTMSG as u8, 0, 0).as_slice())
            .unwrap()
            .hint,
        Hint::SelectMessage(0)
    );
}

#[test]
fn rejects_truncated_messages_invalid_lengths_and_players() {
    let valid = bytes(1, 0, 27);
    for length in 0..valid.len() {
        assert!(CoreMessage::try_from(&valid[..length]).is_err());
    }
    for length in [0, 10, 12, u32::MAX] {
        let mut invalid = valid.clone();
        invalid[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(invalid.as_slice()).is_err());
    }
    for player in 2..=u8::MAX {
        assert!(CoreMessage::try_from(bytes(1, player, 27).as_slice()).is_err());
    }
}

#[test]
fn parses_hint_following_phase_in_same_buffer() {
    let mut buffer = vec![3, 0, 0, 0, 41, 1, 0];
    buffer.extend(bytes(1, 0, 27));
    assert!(matches!(
        parse_messages(&buffer).unwrap().as_slice(),
        [
            CoreMessage::NewPhase(_),
            CoreMessage::Hint(HintMessageData {
                player: Player::Player,
                hint: Hint::Event(27),
            })
        ]
    ));
}
