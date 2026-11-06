use super::*;
use crate::ocgcore::constants::Phase;

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
