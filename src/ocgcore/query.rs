use anyhow::{Context, ensure};
use ocgcore_ffi::constants::{
    QUERY_ATTACK, QUERY_CODE, QUERY_DEFENSE, QUERY_END, QUERY_LEVEL, QUERY_LINK, QUERY_POSITION,
    QUERY_RANK, QUERY_TYPE,
};

use super::constants::{BattlePosition, CardLocation, Player};
use super::data::{CardData, CardType};
use super::utility::{read_u16, read_u32};

pub(super) fn decode_location(
    bytes: &[u8],
    controller: Player,
    location: CardLocation,
) -> anyhow::Result<Vec<Option<CardData>>> {
    let mut cursor = 0;
    let payload_len = usize::try_from(read_u32(bytes, &mut cursor, "query payload length")?)?;
    let end = cursor
        .checked_add(payload_len)
        .context("Query payload length overflow")?;
    let payload = bytes.get(cursor..end).context("Truncated query payload")?;
    let mut cursor = 0;
    let mut cards = Vec::new();

    while cursor < payload.len() {
        let sequence = u8::try_from(cards.len()).context("Too many cards in query location")?;
        let length = usize::from(read_u16(payload, &mut cursor, "query record length")?);
        if length == 0 {
            cards.push(None);
            continue;
        }

        let mut card = CardData {
            controller,
            location,
            sequence,
            ..Default::default()
        };
        let mut length = length;
        loop {
            ensure!(length >= 4, "Query record is shorter than its flag");
            let record_end = cursor
                .checked_add(length)
                .context("Query record length overflow")?;
            let record = payload
                .get(cursor..record_end)
                .context("Truncated query record")?;
            let mut record_cursor = 0;
            let flag = read_u32(record, &mut record_cursor, "query flag")?;

            match flag {
                QUERY_CODE => card.card_code = read_u32(record, &mut record_cursor, "card code")?,
                QUERY_POSITION => {
                    let position = read_u32(record, &mut record_cursor, "card position")?;
                    card.position = BattlePosition::try_from(position).ok();
                }
                QUERY_TYPE => {
                    let card_type = read_u32(record, &mut record_cursor, "card type")?;
                    card.card_type = CardType::from_bits_truncate(card_type);
                }
                QUERY_LEVEL => card.level = Some(read_u32(record, &mut record_cursor, "level")?),
                QUERY_RANK => card.rank = Some(read_u32(record, &mut record_cursor, "rank")?),
                QUERY_ATTACK => card.attack = Some(read_u32(record, &mut record_cursor, "attack")?),
                QUERY_DEFENSE => {
                    card.defense = Some(read_u32(record, &mut record_cursor, "defense")?)
                }
                QUERY_LINK => {
                    card.link_rating = Some(read_u32(record, &mut record_cursor, "link rating")?);
                    read_u32(record, &mut record_cursor, "link markers")?;
                }
                QUERY_END => {
                    ensure!(length == 4, "Invalid QUERY_END record length");
                    cursor = record_end;
                    cards.push(Some(card));
                    break;
                }
                _ => {}
            }

            cursor = record_end;
            length = usize::from(
                read_u16(payload, &mut cursor, "next query record length")
                    .context("Missing QUERY_END")?,
            );
            ensure!(length != 0, "Empty slot encountered before QUERY_END");
        }
    }

    Ok(cards)
}

#[cfg(test)]
mod tests {
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
}
