#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectYesNoMessageData {
    pub player: u8,
    pub card_code: u32,
    pub string_index: usize,
}

impl TryFrom<&[u8]> for SelectYesNoMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[5];

        let description_id = u64::from_le_bytes([
            bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
        ]);

        let card_code = ((description_id >> 20) & 0xffff_ffff) as u32;
        let string_index = (description_id & 0xfffff) as usize;

        Ok(SelectYesNoMessageData {
            player,
            card_code,
            string_index,
        })
    }
}
