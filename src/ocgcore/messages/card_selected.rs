use crate::ocgcore::location_info::CardLocationInfo;

use super::card_locations::read_card_locations;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct CardSelectedMessageData {
    pub cards: Vec<CardLocationInfo>,
}

impl TryFrom<&[u8]> for CardSelectedMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            cards: read_card_locations(bytes, "card selected")?,
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

    const SAMPLE: [u8; 19] = [15, 0, 0, 0, 80, 1, 0, 0, 0, 0, 4, 5, 0, 0, 0, 1, 0, 0, 0];

    #[test]
    fn parses_sample_selected_card() {
        assert_eq!(
            CoreMessage::try_from(SAMPLE.as_slice()).unwrap(),
            CoreMessage::CardSelected(CardSelectedMessageData {
                cards: vec![CardLocationInfo {
                    controller: Some(Player::Player),
                    location: CardLocationKind::Known(CardLocation::MonsterZone),
                    sequence: 5,
                    position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
                }],
            })
        );
    }

    #[test]
    fn parses_empty_and_multiple_selected_cards() {
        assert_eq!(
            CoreMessage::try_from([5, 0, 0, 0, 80, 0, 0, 0, 0].as_slice()).unwrap(),
            CoreMessage::CardSelected(CardSelectedMessageData { cards: vec![] })
        );
        let mut bytes = SAMPLE.to_vec();
        bytes[..4].copy_from_slice(&25u32.to_le_bytes());
        bytes[5..9].copy_from_slice(&2u32.to_le_bytes());
        bytes.extend([PLAYER_NONE as u8, (LOCATION_MZONE | LOCATION_OVERLAY) as u8]);
        bytes.extend(0x1234_5678u32.to_le_bytes());
        bytes.extend(0x8765_4321u32.to_le_bytes());
        let message = CardSelectedMessageData::try_from(bytes.as_slice()).unwrap();
        assert_eq!(message.cards.len(), 2);
        assert_eq!(
            message.cards[1],
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
    fn parses_card_selection_among_framed_messages() {
        let mut bytes = SAMPLE.to_vec();
        bytes.extend([1, 0, 0, 0, 74]);
        assert!(matches!(
            parse_messages(&bytes).unwrap().as_slice(),
            [
                CoreMessage::CardSelected(CardSelectedMessageData { cards }),
                CoreMessage::ChainEnd
            ] if cards.len() == 1
        ));
    }
}
