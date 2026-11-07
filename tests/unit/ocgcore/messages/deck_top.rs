use super::*;
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{MSG_DECK_TOP, MSG_REVERSE_DECK};

fn sample() -> Vec<u8> {
    let mut bytes = 14u32.to_le_bytes().to_vec();
    bytes.extend([MSG_DECK_TOP as u8, 1]);
    bytes.extend(256u32.to_le_bytes());
    bytes.extend(42u32.to_le_bytes());
    bytes.extend(0x8000_0004u32.to_le_bytes());
    bytes
}

#[test]
fn decodes_deck_top_and_reverse_among_framed_messages() {
    let mut bytes = sample();
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::DeckTop(DeckTopMessageData {
            player: Player::Opponent,
            offset: 256,
            card_code: 42,
            position: CardPositionFlags::from_bits_retain(0x8000_0004),
        })
    );
    bytes.extend([1, 0, 0, 0, MSG_REVERSE_DECK as u8]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [CoreMessage::DeckTop(_), CoreMessage::ReverseDeck]
    ));
}

#[test]
fn rejects_truncated_and_invalid_deck_messages() {
    let sample = sample();
    for length in 0..sample.len() {
        assert!(CoreMessage::try_from(&sample[..length]).is_err());
    }
    for length in [0u32, 13, 15, u32::MAX] {
        let mut bytes = sample.clone();
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    let mut bytes = sample;
    bytes[5] = 2;
    assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    for length in [0u32, 2, u32::MAX] {
        let mut bytes = [1, 0, 0, 0, MSG_REVERSE_DECK as u8];
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
}
