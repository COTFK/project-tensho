use super::*;
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{MSG_REVERSE_DECK, MSG_SWAP_GRAVE_DECK};

fn sample() -> Vec<u8> {
    let mut bytes = 12u32.to_le_bytes().to_vec();
    bytes.extend([MSG_SWAP_GRAVE_DECK as u8, 1]);
    bytes.extend(256u32.to_le_bytes());
    bytes.extend(2u32.to_le_bytes());
    bytes.extend([0x81, 0x01]);
    bytes
}

#[test]
fn decodes_exchange_and_combined_frames() {
    let mut bytes = sample();
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::SwapGraveDeck(SwapGraveDeckMessageData {
            player: Player::Opponent,
            extra_insert_index: 256,
            extra_cards: vec![0x81, 0x01],
        })
    );
    bytes.extend([1, 0, 0, 0, MSG_REVERSE_DECK as u8]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [CoreMessage::SwapGraveDeck(_), CoreMessage::ReverseDeck]
    ));
}

#[test]
fn rejects_truncated_and_malformed_exchange() {
    let sample = sample();
    for length in 0..sample.len() {
        assert!(CoreMessage::try_from(&sample[..length]).is_err());
    }
    let mut bytes = sample.clone();
    bytes[5] = 2;
    assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    for length in [0u32, 1, 11, 13, u32::MAX] {
        let mut bytes = sample.clone();
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    let mut bytes = sample;
    bytes[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
}

#[test]
fn decodes_empty_bitmap() {
    let bytes = [
        10,
        0,
        0,
        0,
        MSG_SWAP_GRAVE_DECK as u8,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    assert!(matches!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::SwapGraveDeck(data) if data.extra_cards.is_empty()
    ));
}
