use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShuffleHandMessageData {
    pub player: Player,
    pub card_codes: Vec<u32>,
}

impl TryFrom<&[u8]> for ShuffleHandMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "shuffle player")?)?;
        let count = read_u32(bytes, &mut cursor, "shuffled card count")?;
        let expected_length = count
            .checked_mul(4)
            .and_then(|size| size.checked_add(6))
            .context("Shuffled card count exceeds message size")?;
        anyhow::ensure!(
            length == expected_length,
            "Invalid shuffle hand message length: {length}, expected {expected_length}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Shuffle hand message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated shuffle hand message");

        let mut card_codes = Vec::with_capacity(count as usize);
        for _ in 0..count {
            card_codes.push(read_u32(bytes, &mut cursor, "shuffled card code")?);
        }
        Ok(Self { player, card_codes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocgcore::messages::{CoreMessage, parse_messages};

    const SAMPLE: [u8; 30] = [
        26, 0, 0, 0, 33, 0, 5, 0, 0, 0, 178, 50, 204, 5, 178, 50, 204, 5, 122, 125, 228, 3, 75,
        242, 103, 2, 136, 86, 166, 2,
    ];

    #[test]
    fn parses_sample_shuffled_hand_in_order() {
        assert_eq!(
            CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
            CoreMessage::ShuffleHand(ShuffleHandMessageData {
                player: Player::Player,
                card_codes: vec![97_268_402, 97_268_402, 65_305_978, 40_366_667, 44_455_560],
            })
        );
    }

    #[test]
    fn preserves_hidden_codes_and_empty_hands() {
        let mut bytes = SAMPLE;
        bytes[5] = 1;
        bytes[10..14].copy_from_slice(&0u32.to_le_bytes());
        bytes[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
        let message = ShuffleHandMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(message.player, Player::Opponent);
        assert_eq!(message.card_codes[..2], [0, u32::MAX]);
        assert_eq!(
            CoreMessage::try_from([6, 0, 0, 0, 33, 0, 0, 0, 0, 0].as_slice()).unwrap(),
            CoreMessage::ShuffleHand(ShuffleHandMessageData {
                player: Player::Player,
                card_codes: vec![],
            })
        );
    }

    #[test]
    fn rejects_truncated_frames_and_invalid_data() {
        for length in 0..SAMPLE.len() {
            assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
            assert!(parse_messages(&SAMPLE[..length]).is_err() || length == 0);
        }
        for length in [0u32, 6, 25, 27, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
            assert!(parse_messages(&bytes).is_err());
        }
        for count in [0u32, 4, 6, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[6..10].copy_from_slice(&count.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        }
        for player in 2..=u8::MAX {
            let mut bytes = SAMPLE;
            bytes[5] = player;
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        }
    }

    #[test]
    fn parses_shuffled_hand_among_framed_messages() {
        let mut bytes = vec![3, 0, 0, 0, 41, 1, 0];
        bytes.extend(SAMPLE);
        bytes.extend([1, 0, 0, 0, 63]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [
                CoreMessage::NewPhase(_),
                CoreMessage::ShuffleHand(ShuffleHandMessageData { player: Player::Player, card_codes }),
                CoreMessage::SpecialSummoned
            ] if card_codes.len() == 5
        ));
    }
}
