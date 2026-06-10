#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub card_code: Option<u32>,
    pub string_index: Option<usize>,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SelectOptionMessageData {
    pub player: u8,
    pub options: Vec<SelectOption>,
}

impl TryFrom<&[u8]> for SelectOptionMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[5];

        let count = bytes[6] as usize;
        let mut options = Vec::with_capacity(count);

        for index in 0..count {
            let offset = 7 + (index * 8);
            let raw = u64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);

            // OCG packs effect description ids as: (card_code << 20) | string_index.
            // Small values (e.g. 1, 10, 11, 12) are engine-level option constants.
            let (card_code, string_index) = if raw >= (1 << 20) {
                (
                    Some(((raw >> 20) & 0xffff_ffff) as u32),
                    Some((raw & 0x000f_ffff) as usize),
                )
            } else {
                (None, None)
            };

            options.push(SelectOption {
                card_code,
                string_index,
            });
        }

        Ok(SelectOptionMessageData { player, options })
    }
}
