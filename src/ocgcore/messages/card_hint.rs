use crate::ocgcore::location_info::CardLocationInfo;
use crate::ocgcore::utility::{read_u8, read_u32, read_u64};
use ocgcore_ffi::constants::{
    CHINT_ATTRIBUTE, CHINT_CARD, CHINT_DESC_ADD, CHINT_DESC_REMOVE, CHINT_NUMBER, CHINT_RACE,
    CHINT_TURN,
};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardHint {
    Turn(u64),
    Card(u64),
    Race(u64),
    Attribute(u64),
    Number(u64),
    DescriptionAdd(u64),
    DescriptionRemove(u64),
    Unknown { kind: u8, value: u64 },
}

impl CardHint {
    fn from_wire(kind: u8, value: u64) -> Self {
        match u32::from(kind) {
            CHINT_TURN => Self::Turn(value),
            CHINT_CARD => Self::Card(value),
            CHINT_RACE => Self::Race(value),
            CHINT_ATTRIBUTE => Self::Attribute(value),
            CHINT_NUMBER => Self::Number(value),
            CHINT_DESC_ADD => Self::DescriptionAdd(value),
            CHINT_DESC_REMOVE => Self::DescriptionRemove(value),
            _ => Self::Unknown { kind, value },
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CardHintMessageData {
    pub location: CardLocationInfo,
    pub hint: CardHint,
}

impl TryFrom<&[u8]> for CardHintMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 20, "Invalid card hint message length: {length}");
        read_u8(bytes, &mut cursor, "message type")?;
        let location = CardLocationInfo::read(bytes, &mut cursor)?;
        let kind = read_u8(bytes, &mut cursor, "card hint type")?;
        let value = read_u64(bytes, &mut cursor, "card hint value")?;
        Ok(Self {
            location,
            hint: CardHint::from_wire(kind, value),
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
    use ocgcore_ffi::constants::{LOCATION_MZONE, LOCATION_OVERLAY, PLAYER_NONE};

    const SAMPLE: [u8; 24] = [
        20, 0, 0, 0, 160, 0, 2, 4, 0, 0, 0, 10, 0, 0, 0, 6, 225, 0, 0, 0, 0, 0, 0, 0,
    ];

    #[test]
    fn parses_sample_card_hint() {
        assert_eq!(
            CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
            CoreMessage::CardHint(CardHintMessageData {
                location: CardLocationInfo {
                    controller: Some(Player::Player),
                    location: CardLocationKind::Known(CardLocation::Hand),
                    sequence: 4,
                    position: CardLocationPosition::Position(
                        CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE,
                    ),
                },
                hint: CardHint::DescriptionAdd(225),
            })
        );
    }

    #[test]
    fn parses_all_hint_types_and_preserves_values() {
        let value = 0xfedc_ba98_7654_3210u64;
        for (kind, hint) in [
            (CHINT_TURN, CardHint::Turn(value)),
            (CHINT_CARD, CardHint::Card(value)),
            (CHINT_RACE, CardHint::Race(value)),
            (CHINT_ATTRIBUTE, CardHint::Attribute(value)),
            (CHINT_NUMBER, CardHint::Number(value)),
            (CHINT_DESC_ADD, CardHint::DescriptionAdd(value)),
            (CHINT_DESC_REMOVE, CardHint::DescriptionRemove(value)),
        ] {
            let mut bytes = SAMPLE;
            bytes[15] = kind as u8;
            bytes[16..24].copy_from_slice(&value.to_le_bytes());
            assert_eq!(
                CardHintMessageData::try_from(bytes.as_slice())
                    .unwrap()
                    .hint,
                hint
            );
        }
        for kind in [0, 8, u8::MAX] {
            let mut bytes = SAMPLE;
            bytes[15] = kind;
            bytes[16..24].copy_from_slice(&value.to_le_bytes());
            assert_eq!(
                CardHintMessageData::try_from(bytes.as_slice())
                    .unwrap()
                    .hint,
                CardHint::Unknown { kind, value }
            );
        }
        let mut bytes = SAMPLE;
        bytes[16..24].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(
            CardHintMessageData::try_from(bytes.as_slice())
                .unwrap()
                .hint,
            CardHint::DescriptionAdd(0)
        );
    }

    #[test]
    fn preserves_full_width_and_overlay_locations() {
        let mut bytes = SAMPLE;
        bytes[5] = PLAYER_NONE as u8;
        bytes[6] = (LOCATION_MZONE | LOCATION_OVERLAY) as u8;
        bytes[7..11].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        bytes[11..15].copy_from_slice(&0x8765_4321u32.to_le_bytes());
        assert_eq!(
            CardHintMessageData::try_from(bytes.as_slice())
                .unwrap()
                .location,
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
    fn rejects_truncated_messages_invalid_lengths_and_locations() {
        for length in 0..SAMPLE.len() {
            assert!(CoreMessage::try_from(&SAMPLE[..length]).is_err());
            assert!(parse_messages(&SAMPLE[..length]).is_err() || length == 0);
        }
        for length in [0u32, 19, 21, u32::MAX] {
            let mut bytes = SAMPLE;
            bytes[..4].copy_from_slice(&length.to_le_bytes());
            assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
            assert!(parse_messages(&bytes).is_err());
        }
        let mut bytes = SAMPLE;
        bytes[5] = 3;
        assert!(CoreMessage::try_from(bytes.as_slice()).is_err());
        for location in [0x0c, 0x8c, 0xff] {
            let mut bytes = SAMPLE;
            bytes[6] = location;
            let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
            assert!(format!("{error:#}").contains(&format!("{location:#04x}")));
        }
    }

    #[test]
    fn parses_card_hint_among_framed_messages() {
        let mut bytes = vec![3, 0, 0, 0, 41, 1, 0];
        bytes.extend(SAMPLE);
        bytes.extend([1, 0, 0, 0, 63]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [
                CoreMessage::NewPhase(_),
                CoreMessage::CardHint(CardHintMessageData {
                    hint: CardHint::DescriptionAdd(225),
                    ..
                }),
                CoreMessage::SpecialSummoned
            ]
        ));
    }
}
