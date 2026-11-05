use anyhow::Context;

use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct BecomeTargetMessageData {
    pub targets: Vec<CardLocationInfo>,
}

impl TryFrom<&[u8]> for BecomeTargetMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        read_u8(bytes, &mut cursor, "message type")?;
        let count = read_u32(bytes, &mut cursor, "target count")?;
        let expected_length = count
            .checked_mul(10)
            .and_then(|size| size.checked_add(5))
            .context("Target count exceeds message size")?;
        anyhow::ensure!(
            length == expected_length,
            "Invalid become target message length: {length}, expected {expected_length}"
        );
        let end = usize::try_from(length)?
            .checked_add(4)
            .context("Become target message length overflow")?;
        anyhow::ensure!(bytes.len() >= end, "Truncated become target message");

        let mut targets = Vec::with_capacity(count as usize);
        for _ in 0..count {
            targets.push(CardLocationInfo::read(bytes, &mut cursor)?);
        }
        Ok(Self { targets })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocgcore::constants::{CardLocation, Player};
    use crate::ocgcore::location_info::{
        CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::{CoreMessage, parse_messages};
    use ocgcore_ffi::constants::{LOCATION_MZONE, LOCATION_OVERLAY, PLAYER_NONE};

    const SAMPLE: [u8; 19] = [15, 0, 0, 0, 83, 1, 0, 0, 0, 0, 8, 5, 0, 0, 0, 5, 0, 0, 0];

    #[test]
    fn parses_sample_target() {
        assert_eq!(
            CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
            CoreMessage::BecomeTarget(BecomeTargetMessageData {
                targets: vec![CardLocationInfo {
                    controller: Some(Player::Player),
                    location: CardLocationKind::Known(CardLocation::SpellTrapZone),
                    sequence: 5,
                    position: CardLocationPosition::Position(
                        CardPositionFlags::FACE_UP_ATTACK | CardPositionFlags::FACE_UP_DEFENSE,
                    ),
                }],
            })
        );
    }

    #[test]
    fn parses_empty_and_multiple_targets_with_full_width_fields() {
        let empty = [5, 0, 0, 0, 83, 0, 0, 0, 0];
        assert_eq!(
            CoreMessage::try_from(empty.as_slice()).unwrap(),
            CoreMessage::BecomeTarget(BecomeTargetMessageData { targets: vec![] })
        );
        let mut bytes = SAMPLE.to_vec();
        bytes[..4].copy_from_slice(&25u32.to_le_bytes());
        bytes[5..9].copy_from_slice(&2u32.to_le_bytes());
        bytes.extend([PLAYER_NONE as u8, (LOCATION_MZONE | LOCATION_OVERLAY) as u8]);
        bytes.extend(0x1234_5678u32.to_le_bytes());
        bytes.extend(0x8765_4321u32.to_le_bytes());
        let message = BecomeTargetMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(message.targets.len(), 2);
        assert_eq!(
            message.targets[1],
            CardLocationInfo {
                controller: None,
                location: CardLocationKind::Overlay {
                    host: Some(CardLocation::MonsterZone),
                },
                sequence: 0x1234_5678,
                position: CardLocationPosition::OverlaySequence(0x8765_4321),
            }
        );
    }

    #[test]
    fn rejects_truncated_messages_invalid_lengths_and_targets() {
        for length in 0..SAMPLE.len() {
            assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
            assert!(parse_messages(&SAMPLE[..length]).is_err() || length == 0);
        }
        for length in [0u32, 5, 14, 16, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
            assert!(parse_messages(&bytes).is_err());
        }
        for count in [0u32, 2, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[5..9].copy_from_slice(&count.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        }
        let mut bytes = SAMPLE;
        bytes[9] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        for location in [0x0c, 0x8c, 0xff] {
            let mut bytes = SAMPLE;
            bytes[10] = location;
            let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
            assert!(format!("{error:#}").contains(&format!("{location:#04x}")));
        }
    }

    #[test]
    fn parses_targets_among_framed_messages() {
        let mut bytes = SAMPLE.to_vec();
        bytes.extend([1, 0, 0, 0, 74]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [
                CoreMessage::BecomeTarget(BecomeTargetMessageData { targets }),
                CoreMessage::ChainEnd
            ] if targets.len() == 1
        ));
    }
}
