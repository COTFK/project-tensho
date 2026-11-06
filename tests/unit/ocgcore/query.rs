use super::*;
use ocgcore_ffi::constants::{POS_FACEUP_DEFENSE, QUERY_IS_PUBLIC};

fn record(flag: u32, data: &[u8]) -> Vec<u8> {
    let length = u16::try_from(4 + data.len()).unwrap();
    [length.to_le_bytes().as_slice(), &flag.to_le_bytes(), data].concat()
}

fn number(flag: u32, value: u32) -> Vec<u8> {
    record(flag, &value.to_le_bytes())
}

fn frame(payload: &[u8]) -> Vec<u8> {
    [
        u32::try_from(payload.len())
            .unwrap()
            .to_le_bytes()
            .as_slice(),
        payload,
    ]
    .concat()
}

fn decode(bytes: &[u8]) -> anyhow::Result<Vec<Option<CardData>>> {
    decode_location(bytes, Player::Opponent, CardLocation::MonsterZone)
}

#[test]
fn decodes_empty_locations_and_slots() {
    assert_eq!(decode(&frame(&[])).unwrap(), vec![]);
    assert_eq!(decode(&frame(&[0, 0, 0, 0])).unwrap(), vec![None, None]);
}

#[test]
fn decodes_properties_and_preserves_slot_order() {
    let payload = [
        vec![0, 0],
        number(QUERY_CODE, 12345),
        number(QUERY_POSITION, POS_FACEUP_DEFENSE),
        number(QUERY_TYPE, CardType::MONSTER.bits() | CardType::LINK.bits()),
        number(QUERY_LEVEL, 4),
        number(QUERY_RANK, 3),
        number(QUERY_ATTACK, 2400),
        number(QUERY_DEFENSE, u32::MAX),
        record(QUERY_LINK, &[2, 0, 0, 0, 0x41, 0, 0, 0]),
        record(QUERY_IS_PUBLIC, &[1]),
        record(QUERY_END, &[]),
        vec![0, 0],
        number(QUERY_CODE, 12345),
        record(QUERY_END, &[]),
    ]
    .concat();

    assert_eq!(
        decode(&frame(&payload)).unwrap(),
        vec![
            None,
            Some(CardData {
                card_code: 12345,
                controller: Player::Opponent,
                location: CardLocation::MonsterZone,
                position: Some(BattlePosition::FaceUpDefense),
                sequence: 1,
                level: Some(4),
                rank: Some(3),
                attack: Some(2400),
                defense: Some(u32::MAX),
                card_type: CardType::MONSTER | CardType::LINK,
                link_rating: Some(2),
                ..Default::default()
            }),
            None,
            Some(CardData {
                card_code: 12345,
                controller: Player::Opponent,
                location: CardLocation::MonsterZone,
                sequence: 3,
                ..Default::default()
            }),
        ]
    );
}

#[test]
fn preserves_unknown_values_and_ignores_unknown_records() {
    let payload = [
        number(QUERY_POSITION, 0xffff_ffff),
        number(QUERY_TYPE, CardType::SPELL.bits() | 0x8000_0000),
        record(0x4000_0000, &[1, 2, 3]),
        record(QUERY_END, &[]),
    ]
    .concat();
    let cards = decode(&frame(&payload)).unwrap();
    let card = cards[0].unwrap();
    assert_eq!(card.position, None);
    assert_eq!(card.card_type, CardType::SPELL);
    assert_eq!(card.card_code, 0);
    assert_eq!(card.level, None);
    assert_eq!(card.link_rating, None);
}

#[test]
fn rejects_truncated_headers_and_payloads() {
    let bytes = frame(&[0, 0]);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err(), "accepted prefix {end}");
    }
    assert!(decode(&[0xff, 0xff, 0xff, 0xff]).is_err());
    assert_eq!(decode(&[0, 0, 0, 0, 99]).unwrap(), vec![]);
}

#[test]
fn rejects_truncated_records_and_missing_terminators() {
    let valid = [number(QUERY_CODE, 42), record(QUERY_END, &[])].concat();
    for end in 1..valid.len() {
        assert!(
            decode(&frame(&valid[..end])).is_err(),
            "accepted prefix {end}"
        );
    }
    for payload in [
        vec![1, 0, 0],
        vec![3, 0, 0, 0, 0],
        vec![4, 0, 1, 0, 0, 0, 0, 0],
        record(QUERY_END, &[0]),
    ] {
        assert!(decode(&frame(&payload)).is_err(), "accepted {payload:?}");
    }
}

#[test]
fn rejects_short_known_values_without_reading_the_next_record() {
    for flag in [
        QUERY_CODE,
        QUERY_POSITION,
        QUERY_TYPE,
        QUERY_LEVEL,
        QUERY_RANK,
        QUERY_ATTACK,
        QUERY_DEFENSE,
        QUERY_LINK,
    ] {
        for len in 0..4 {
            let payload = [record(flag, &vec![0; len]), record(QUERY_END, &[])].concat();
            assert!(decode(&frame(&payload)).is_err(), "accepted flag {flag:#x}");
        }
    }
    let payload = [number(QUERY_LINK, 2), record(QUERY_END, &[])].concat();
    assert!(decode(&frame(&payload)).is_err());
}

#[test]
fn rejects_sequences_that_do_not_fit_card_data() {
    let slots = vec![0; 256 * 2];
    assert_eq!(decode(&frame(&slots)).unwrap().len(), 256);
    let mut too_many = slots;
    too_many.extend([0, 0]);
    assert!(decode(&frame(&too_many)).is_err());

    let payload = [vec![0; 255 * 2], record(QUERY_END, &[])].concat();
    assert_eq!(
        decode(&frame(&payload)).unwrap()[255].unwrap().sequence,
        255
    );
}
