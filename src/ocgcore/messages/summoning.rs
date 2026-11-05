use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SummoningMessageData {
    pub card_code: u32,
    pub location: CardLocationInfo,
}

impl TryFrom<&[u8]> for SummoningMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 15, "Invalid summoning message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let card_code = read_u32(bytes, &mut cursor, "card code")?;
        let location = CardLocationInfo::read(bytes, &mut cursor)?;
        Ok(Self {
            card_code,
            location,
        })
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

    const SAMPLE: [u8; 19] = [
        15, 0, 0, 0, 60, 0, 175, 103, 5, 0, 4, 2, 0, 0, 0, 1, 0, 0, 0,
    ];

    #[test]
    fn parses_sample_summoning() {
        assert_eq!(
            CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
            CoreMessage::Summoning(SummoningMessageData {
                card_code: 90_681_088,
                location: CardLocationInfo {
                    controller: Some(Player::Player),
                    location: CardLocationKind::Known(CardLocation::MonsterZone),
                    sequence: 2,
                    position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
                },
            })
        );
    }

    #[test]
    fn preserves_full_width_location_values() {
        let mut bytes = SAMPLE;
        bytes[9] = 1;
        bytes[11..15].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        bytes[15..19].copy_from_slice(&0x8000_0004u32.to_le_bytes());
        let message = SummoningMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(message.location.controller, Some(Player::Opponent));
        assert_eq!(message.location.sequence, 0x1234_5678);
        assert_eq!(
            message.location.position,
            CardLocationPosition::Position(CardPositionFlags::from_bits_retain(0x8000_0004))
        );
        bytes[9] = ocgcore_ffi::constants::PLAYER_NONE as u8;
        assert_eq!(
            SummoningMessageData::try_from(bytes.as_slice())
                .unwrap()
                .location
                .controller,
            None
        );
    }

    #[test]
    fn rejects_truncated_messages_invalid_lengths_and_controllers() {
        for length in 0..SAMPLE.len() {
            assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
        }
        for length in [0u32, 14, 16, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        }
        let mut bytes = SAMPLE;
        bytes[9] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        for location in [0x0c, 0x8c, 0xff] {
            let mut bytes = SAMPLE;
            bytes[10] = location;
            let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
            assert!(
                format!("{error:#}").contains(&format!("{location:#04x}")),
                "error did not include raw location: {error:#}"
            );
        }
    }

    #[test]
    fn parses_multiple_framed_messages() {
        let mut bytes = SAMPLE.to_vec();
        bytes.extend([3, 0, 0, 0, 41, 1, 0]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [CoreMessage::Summoning(_), CoreMessage::NewPhase(_)]
        ));
    }
}
