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
        }
    }

    Ok(cards)
}

#[cfg(test)]
#[path = "../../tests/unit/ocgcore/query.rs"]
mod tests;
