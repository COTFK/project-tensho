use super::*;
use crate::ocgcore::constants::Phase;

#[test]
fn parses_large_select_unselect_lists_and_rejects_truncation() {
    let mut bytes = vec![0, 0, 0, 0, MSG_SELECT_UNSELECT_CARD as u8, 0, 1, 1];
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&101u32.to_le_bytes());
    for _ in 0..2 {
        bytes.extend_from_slice(&101u32.to_le_bytes());
        for sequence in 0..101u32 {
            bytes.extend_from_slice(&42u32.to_le_bytes());
            bytes.push(0);
            bytes.push(crate::ocgcore::constants::CardLocation::Deck as u8);
            bytes.extend_from_slice(&sequence.to_le_bytes());
            bytes.extend_from_slice(&1u32.to_le_bytes());
        }
    }
    let length = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&length.to_le_bytes());
    let CoreMessage::SelectUnselectCard(message) = CoreMessage::try_from(bytes.as_slice()).unwrap()
    else {
        panic!("Expected select-unselect message");
    };
    assert_eq!(message.select_cards.len(), 101);
    assert_eq!(message.unselect_cards.len(), 101);
    assert!(message.unselect_cards.iter().all(|card| card.is_selected));
    bytes.pop();
    assert!(SelectUnselectMessageData::try_from(bytes.as_slice()).is_err());
}

#[test]
fn parses_large_idle_lists_and_skips_position_records() {
    let mut bytes = vec![0, 0, 0, 0, MSG_SELECT_IDLECMD as u8, 0];
    bytes.extend_from_slice(&101u32.to_le_bytes());
    for sequence in 0..101u32 {
        bytes.extend_from_slice(&42u32.to_le_bytes());
        bytes.extend_from_slice(&[0, crate::ocgcore::constants::CardLocation::Hand as u8]);
        bytes.extend_from_slice(&sequence.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 0, 0, 0, 0, 4, 3]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&77u32.to_le_bytes());
    bytes.extend_from_slice(&[0, crate::ocgcore::constants::CardLocation::Hand as u8]);
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&99u32.to_le_bytes());
    bytes.extend_from_slice(&[0, crate::ocgcore::constants::CardLocation::Graveyard as u8]);
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&0x123456u64.to_le_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&[0, 1, 0]);
    let length = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&length.to_le_bytes());

    let CoreMessage::Idle(idle) = CoreMessage::try_from(bytes.as_slice()).unwrap() else {
        panic!("Expected idle message");
    };
    assert_eq!(idle.normal_summons.len(), 101);
    assert_eq!(idle.normal_summons[100].action_index, Some(100));
    assert_eq!(idle.monster_sets[0].card_code, 77);
    assert_eq!(idle.activatable_effects[0].description, Some(0x23456));
    assert_eq!(idle.activatable_effects[0].sequence, 3);
    for end in 0..bytes.len() {
        assert!(CoreMessage::try_from(&bytes[..end]).is_err());
    }
}

#[test]
fn parses_framed_messages_in_order() {
    let bytes = [1, 0, 0, 0, 255, 3, 0, 0, 0, 41, 1, 0, 3, 0, 0, 0, 41, 0, 2];
    assert_eq!(
        parse_messages(&bytes).unwrap(),
        vec![
            CoreMessage::Unsupported(255),
            CoreMessage::NewPhase(NewPhaseMessageData { phase: Phase::Draw }),
            CoreMessage::NewPhase(NewPhaseMessageData { phase: Phase::End }),
        ]
    );
    assert!(parse_messages(&[]).unwrap().is_empty());
}

#[test]
fn rejects_invalid_frames_and_truncated_messages() {
    assert!(parse_messages(&[0, 0, 0, 0]).is_err());
    assert!(parse_messages(&[2, 0, 0, 0, 41]).is_err());
    assert!(parse_messages(&[1, 0, 0]).is_err());
    assert!(parse_messages(&[2, 0, 0, 0, 41, 1, 1, 0, 0, 0, 2]).is_err());
}

#[test]
fn parses_chain_end_and_rejects_invalid_lengths() {
    let bytes = [1, 0, 0, 0, 74];
    assert_eq!(
        CoreMessage::try_from(bytes.as_slice()).unwrap(),
        CoreMessage::ChainEnd
    );
    for length in 0..bytes.len() {
        assert!(CoreMessage::try_from(&bytes[..length]).is_err());
        assert!(parse_messages(&bytes[..length]).is_err() || length == 0);
    }
    for length in [0u32, 2, u32::MAX] {
        let mut invalid = bytes;
        invalid[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(invalid.as_slice()).is_err());
    }
    assert!(parse_messages(&[2, 0, 0, 0, 74, 0]).is_err());
    assert_eq!(
        parse_messages(&[2, 0, 0, 0, 73, 2, 1, 0, 0, 0, 74]).unwrap(),
        vec![
            CoreMessage::ChainSolved(ChainLinkMessageData { chain_link: 2 }),
            CoreMessage::ChainEnd,
        ]
    );
}

#[test]
fn parses_summoned_and_rejects_invalid_lengths() {
    for (bytes, expected) in [
        ([1, 0, 0, 0, 61], CoreMessage::Summoned),
        ([1, 0, 0, 0, 63], CoreMessage::SpecialSummoned),
    ] {
        assert_eq!(CoreMessage::try_from(bytes.as_slice()).unwrap(), expected);
        for length in 0..bytes.len() {
            assert!(CoreMessage::try_from(&bytes[..length]).is_err());
        }
        for length in [0u32, 2, u32::MAX] {
            let mut invalid = bytes;
            invalid[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(invalid.as_slice()).is_err());
        }
        assert!(parse_messages(&[2, 0, 0, 0, bytes[4], 0]).is_err());
    }
    assert_eq!(
        parse_messages(&[1, 0, 0, 0, 61, 1, 0, 0, 0, 63, 3, 0, 0, 0, 41, 1, 0]).unwrap(),
        vec![
            CoreMessage::Summoned,
            CoreMessage::SpecialSummoned,
            CoreMessage::NewPhase(NewPhaseMessageData { phase: Phase::Draw }),
        ]
    );
}
