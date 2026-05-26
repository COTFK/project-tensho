use anyhow::Context;

pub fn read_u8(raw_bytes: &[u8], cursor: &mut usize, label: &str) -> anyhow::Result<u8> {
    let value = raw_bytes
        .get(*cursor)
        .copied()
        .with_context(|| format!("Missing {label} at offset {}", *cursor))?;
    *cursor += 1;
    Ok(value)
}

pub fn read_u32(raw_bytes: &[u8], cursor: &mut usize, label: &str) -> anyhow::Result<u32> {
    let bytes = raw_bytes
        .get(*cursor..*cursor + 4)
        .with_context(|| format!("Missing {label} at offset {}", *cursor))?;
    *cursor += 4;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

pub fn read_u64(raw_bytes: &[u8], cursor: &mut usize, label: &str) -> anyhow::Result<u64> {
    let bytes = raw_bytes
        .get(*cursor..*cursor + 8)
        .with_context(|| format!("Missing {label} at offset {}", *cursor))?;
    *cursor += 8;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}
