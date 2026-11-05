use crate::ocgcore::constants::Player;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShuffleDeckMessageData {
    pub player: Player,
}

impl TryFrom<&[u8]> for ShuffleDeckMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 2, "Invalid shuffle deck message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "shuffle player")?)?;
        Ok(Self { player })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocgcore::messages::{CoreMessage, parse_messages};

    #[test]
    fn parses_shuffle_deck_for_both_players() {
        for (value, player) in [(0, Player::Player), (1, Player::Opponent)] {
            let bytes = [2, 0, 0, 0, 32, value];
            assert_eq!(
                CoreMessage::try_from(bytes.as_slice()).unwrap(),
                CoreMessage::ShuffleDeck(ShuffleDeckMessageData { player })
            );
        }
    }

    #[test]
    fn rejects_truncated_messages_invalid_lengths_and_players() {
        let bytes = [2, 0, 0, 0, 32, 0];
        for length in 0..bytes.len() {
            assert!(CoreMessage::try_from(&bytes[..length]).is_err());
            assert!(parse_messages(&bytes[..length]).is_err() || length == 0);
        }
        for length in [0u32, 1, 3, u32::MAX] {
            let mut invalid = bytes;
            invalid[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(invalid.as_slice()).is_err());
            assert!(parse_messages(&invalid).is_err());
        }
        for player in 2..=u8::MAX {
            let mut invalid = bytes;
            invalid[5] = player;
            assert!(CoreMessage::try_from(invalid.as_slice()).is_err());
        }
    }

    #[test]
    fn parses_shuffle_deck_among_framed_messages() {
        let bytes = [2, 0, 0, 0, 32, 0, 3, 0, 0, 0, 41, 1, 0];
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [
                CoreMessage::ShuffleDeck(ShuffleDeckMessageData {
                    player: Player::Player
                }),
                CoreMessage::NewPhase(_)
            ]
        ));
    }
}
