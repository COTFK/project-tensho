#[derive(Hash, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnnounceNumberMessageData {
    pub player: u8,
    pub numbers: Vec<u32>,
}

impl TryFrom<&[u8]> for AnnounceNumberMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let player = bytes[5];
        let count = bytes[6] as usize;
        let mut numbers = Vec::with_capacity(count);

        for index in 0..count {
            let offset = 7 + (index * 8);
            let value = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            numbers.push(value);
        }

        Ok(AnnounceNumberMessageData { player, numbers })
    }
}
