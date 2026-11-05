use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

pub(super) fn read_card_event(
    bytes: &[u8],
    event: &str,
) -> anyhow::Result<(u32, CardLocationInfo)> {
    let mut cursor = 0;
    let length = read_u32(bytes, &mut cursor, "message length")?;
    anyhow::ensure!(length == 15, "Invalid {event} message length: {length}");
    read_u8(bytes, &mut cursor, "message type")?;
    let card_code = read_u32(bytes, &mut cursor, "card code")?;
    let location = CardLocationInfo::read(bytes, &mut cursor)?;
    Ok((card_code, location))
}
