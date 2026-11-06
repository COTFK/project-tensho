use anyhow::Context;

use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrawCard {
    pub code: u32,
    pub position: CardPositionFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrawMessageData {
    pub player: Player,
    pub cards: Vec<DrawCard>,
}

impl TryFrom<&[u8]> for DrawMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let player = Player::try_from(read_u8(bytes, &mut cursor, "draw player")?)?;
        let count = read_u32(bytes, &mut cursor, "draw count")?;
        let expected = count
            .checked_mul(8)
            .and_then(|size| size.checked_add(6))
            .context("Draw count exceeds message size")?;
        anyhow::ensure!(
            length == expected,
            "Invalid draw message length: {length}, expected {expected}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Draw message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated draw message");

        let mut cards = Vec::with_capacity(count as usize);
        for _ in 0..count {
            cards.push(DrawCard {
                code: read_u32(bytes, &mut cursor, "draw card code")?,
                position: CardPositionFlags::from_bits_retain(read_u32(
                    bytes,
                    &mut cursor,
                    "draw card position",
                )?),
            });
        }
        Ok(Self { player, cards })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocgcore::messages::{CoreMessage, parse_messages};
    use ocgcore_ffi::constants::{MSG_DRAW, POS_FACEDOWN, POS_FACEUP};

    fn frame(player: u8, cards: &[(u32, u32)]) -> Vec<u8> {
        let mut payload = vec![MSG_DRAW as u8, player];
        payload.extend_from_slice(&(cards.len() as u32).to_le_bytes());
        for (code, position) in cards {
            payload.extend_from_slice(&code.to_le_bytes());
            payload.extend_from_slice(&position.to_le_bytes());
        }
        [
            u32::try_from(payload.len())
                .unwrap()
                .to_le_bytes()
                .as_slice(),
            &payload,
        ]
        .concat()
    }

    #[test]
    fn decodes_multiple_cards_and_position_flags() {
        let bytes = frame(1, &[(42, POS_FACEDOWN), (0, POS_FACEUP), (99, 0x8000_0000)]);
        assert_eq!(
            DrawMessageData::try_from(bytes.as_slice()).unwrap(),
            DrawMessageData {
                player: Player::Opponent,
                cards: vec![
                    DrawCard {
                        code: 42,
                        position: CardPositionFlags::from_bits_retain(POS_FACEDOWN)
                    },
                    DrawCard {
                        code: 0,
                        position: CardPositionFlags::from_bits_retain(POS_FACEUP)
                    },
                    DrawCard {
                        code: 99,
                        position: CardPositionFlags::from_bits_retain(0x8000_0000)
                    },
                ],
            }
        );
        assert!(
            DrawMessageData::try_from(frame(0, &[]).as_slice())
                .unwrap()
                .cards
                .is_empty()
        );
    }

    #[test]
    fn rejects_truncated_and_inconsistent_buffers() {
        let bytes = frame(0, &[(42, POS_FACEDOWN), (17, POS_FACEUP)]);
        for length in 0..bytes.len() {
            assert!(
                DrawMessageData::try_from(&bytes[..length]).is_err(),
                "{length}"
            );
        }
        for count in [0u32, 1, 3, u32::MAX] {
            let mut invalid = bytes.clone();
            invalid[6..10].copy_from_slice(&count.to_le_bytes());
            assert!(DrawMessageData::try_from(invalid.as_slice()).is_err());
        }
        for length in [0u32, 5, 6, 21, 23, u32::MAX] {
            let mut invalid = bytes.clone();
            invalid[..4].copy_from_slice(&length.to_le_bytes());
            assert!(DrawMessageData::try_from(invalid.as_slice()).is_err());
        }
        for player in [2, 255] {
            let invalid = frame(player, &[]);
            assert!(DrawMessageData::try_from(invalid.as_slice()).is_err());
        }
    }

    #[test]
    fn parses_draw_among_framed_messages() {
        let mut bytes = frame(0, &[(42, POS_FACEDOWN)]);
        bytes.extend([1, 0, 0, 0, 61]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [CoreMessage::Draw(_), CoreMessage::Summoned]
        ));
    }
}
