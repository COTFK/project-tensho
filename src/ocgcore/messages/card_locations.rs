use anyhow::Context;

use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

pub(super) fn read_card_locations(
    bytes: &[u8],
    event: &str,
) -> anyhow::Result<Vec<CardLocationInfo>> {
    let mut cursor = 0;
    let length = read_u32(bytes, &mut cursor, "message length")?;
    read_u8(bytes, &mut cursor, "message type")?;
    let count = read_u32(bytes, &mut cursor, "card count")?;
    let expected_length = count
        .checked_mul(10)
        .and_then(|size| size.checked_add(5))
        .with_context(|| format!("{event} card count exceeds message size"))?;
    anyhow::ensure!(
        length == expected_length,
        "Invalid {event} message length: {length}, expected {expected_length}"
    );
    let end = usize::try_from(length)?
        .checked_add(4)
        .with_context(|| format!("{event} message length overflow"))?;
    anyhow::ensure!(bytes.len() >= end, "Truncated {event} message");

    let mut cards = Vec::with_capacity(count as usize);
    for _ in 0..count {
        cards.push(CardLocationInfo::read(bytes, &mut cursor)?);
    }
    Ok(cards)
}
