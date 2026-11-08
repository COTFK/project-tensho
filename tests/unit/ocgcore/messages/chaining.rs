use super::*;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::location_info::{CardLocationPosition, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, parse_messages};
use ocgcore_ffi::constants::{LOCATION_OVERLAY, PLAYER_NONE};

const SAMPLE: [u8; 37] = [
    33, 0, 0, 0, 70, 136, 86, 166, 2, 0, 4, 2, 0, 0, 0, 1, 0, 0, 0, 0, 4, 2, 0, 0, 0, 0, 0, 128,
    104, 101, 42, 0, 0, 1, 0, 0, 0,
];

#[test]
fn parses_sample_chaining() {
    assert_eq!(
        CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
        CoreMessage::Chaining(ChainingMessageData {
            card_code: 44_455_560,
            location: CardLocationInfo {
                controller: Some(Player::Local),
                location: CardLocationKind::Known(CardLocation::MonsterZone),
                sequence: 2,
                position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
            },
            triggering_location: ChainTriggerLocation {
                controller: Some(Player::Local),
                location: CardLocationKind::Known(CardLocation::MonsterZone),
                sequence: 2,
            },
            description: 46_615_033_282_560,
            chain_link: 1,
        })
    );
}

#[test]
fn preserves_distinct_locations_and_full_width_fields() {
    let mut bytes = SAMPLE;
    bytes[19] = PLAYER_NONE as u8;
    bytes[20] = LOCATION_OVERLAY as u8;
    bytes[21..25].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    bytes[25..33].copy_from_slice(&0xfedc_ba98_7654_3210u64.to_le_bytes());
    bytes[33..37].copy_from_slice(&0x8765_4321u32.to_le_bytes());
    let message = ChainingMessageData::try_from(bytes.as_slice()).unwrap();
    assert_eq!(
        message.location.location,
        CardLocationKind::Known(CardLocation::MonsterZone)
    );
    assert_eq!(
        message.triggering_location,
        ChainTriggerLocation {
            controller: None,
            location: CardLocationKind::Overlay { host: None },
            sequence: 0x1234_5678,
        }
    );
    assert_eq!(message.description, 0xfedc_ba98_7654_3210);
    assert_eq!(message.chain_link, 0x8765_4321);
}

#[test]
fn rejects_truncated_and_invalid_messages() {
    for length in 0..SAMPLE.len() {
        assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
    }
    for length in [0u32, 32, 34, u32::MAX] {
        let mut bytes = SAMPLE;
        bytes[..4].copy_from_slice(&length.to_le_bytes());
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [9, 19] {
        let mut bytes = SAMPLE;
        bytes[offset] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
    }
    for offset in [10, 20] {
        for value in [0x0c, 0x8c, 0xff] {
            let mut bytes = SAMPLE;
            bytes[offset] = value;
            let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
            assert!(format!("{error:#}").contains(&format!("{value:#04x}")));
        }
    }
}

#[test]
fn parses_multiple_framed_messages() {
    let mut bytes = SAMPLE.to_vec();
    bytes.extend([1, 0, 0, 0, 61]);
    assert!(matches!(
        parse_messages(&bytes).unwrap().as_slice(),
        [CoreMessage::Chaining(_), CoreMessage::Summoned]
    ));
}

#[test]
fn parses_chain_link_messages() {
    for chain_link in [0, 1, 2, u8::MAX] {
        for (message_type, expected) in [
            (
                71,
                CoreMessage::Chained(ChainLinkMessageData { chain_link }),
            ),
            (
                72,
                CoreMessage::ChainSolving(ChainLinkMessageData { chain_link }),
            ),
            (
                73,
                CoreMessage::ChainSolved(ChainLinkMessageData { chain_link }),
            ),
            (
                76,
                CoreMessage::ChainDisabled(ChainLinkMessageData { chain_link }),
            ),
        ] {
            let bytes = [2, 0, 0, 0, message_type, chain_link];
            assert_eq!(CoreMessage::try_from(bytes.as_slice()).unwrap(), expected);
        }
    }
    assert_eq!(
        parse_messages(&[
            2, 0, 0, 0, 71, 1, 2, 0, 0, 0, 72, 1, 2, 0, 0, 0, 76, 1, 2, 0, 0, 0, 73, 1,
        ])
        .unwrap(),
        vec![
            CoreMessage::Chained(ChainLinkMessageData { chain_link: 1 }),
            CoreMessage::ChainSolving(ChainLinkMessageData { chain_link: 1 }),
            CoreMessage::ChainDisabled(ChainLinkMessageData { chain_link: 1 }),
            CoreMessage::ChainSolved(ChainLinkMessageData { chain_link: 1 }),
        ]
    );
}

#[test]
fn rejects_invalid_chain_link_messages() {
    for message_type in [71, 72, 73, 76] {
        let bytes = [2, 0, 0, 0, message_type, 1];
        for length in 0..bytes.len() {
            assert!(CoreMessage::try_from(&bytes[..length]).is_err());
            assert!(parse_messages(&bytes[..length]).is_err() || length == 0);
        }
        for length in [0u32, 1, 3, u32::MAX] {
            let mut invalid = bytes;
            invalid[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(invalid.as_slice()).is_err());
            assert!(parse_messages(&invalid).is_err());
        }
    }
}
