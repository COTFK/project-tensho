use crate::ocgcore::constants::WinReason;
use crate::ocgcore::constants::Winner;
use crate::ocgcore::utility::read_u8;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WinMessageData {
    pub winner: Winner,
    pub reason: WinReason,
}

impl TryFrom<&[u8]> for WinMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let mut cursor = 5;
        let winner = Winner::try_from(read_u8(bytes, &mut cursor, "winner")?)?;
        let reason = WinReason::from(read_u8(bytes, &mut cursor, "win reason")?);

        Ok(Self { winner, reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocgcore::constants::Player;
    use crate::ocgcore::messages::CoreMessage;

    #[test]
    fn parses_win_message() {
        let bytes = [3, 0, 0, 0, 5, 0, 2];
        assert_eq!(
            CoreMessage::try_from(bytes.as_slice()).unwrap(),
            CoreMessage::Win(WinMessageData {
                winner: Winner::Player(Player::Player),
                reason: WinReason::DeckOut,
            })
        );
    }

    #[test]
    fn preserves_draw_and_card_defined_reason() {
        let bytes = [3, 0, 0, 0, 5, 2, 0xff];
        assert_eq!(
            CoreMessage::try_from(bytes.as_slice()).unwrap(),
            CoreMessage::Win(WinMessageData {
                winner: Winner::Draw,
                reason: WinReason::Custom(0xff),
            })
        );
    }

    #[test]
    fn parses_all_winners() {
        for (value, winner) in [
            (0, Winner::Player(Player::Player)),
            (1, Winner::Player(Player::Opponent)),
            (2, Winner::Draw),
        ] {
            let bytes = [3, 0, 0, 0, 5, value, 1];
            assert_eq!(
                CoreMessage::try_from(bytes.as_slice()).unwrap(),
                CoreMessage::Win(WinMessageData {
                    winner,
                    reason: WinReason::LifePointsDepleted,
                })
            );
        }
    }

    #[test]
    fn rejects_invalid_winners() {
        for value in 3..=u8::MAX {
            let bytes = [3, 0, 0, 0, 5, value, 1];
            let error = CoreMessage::try_from(bytes.as_slice()).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("Received invalid winner value: {value}")
            );
        }
    }

    #[test]
    fn parses_all_win_reasons() {
        for reason in u8::MIN..=u8::MAX {
            let bytes = [3, 0, 0, 0, 5, 1, reason];
            let expected = match reason {
                1 => WinReason::LifePointsDepleted,
                2 => WinReason::DeckOut,
                value => WinReason::Custom(value),
            };
            assert_eq!(
                WinMessageData::try_from(bytes.as_slice()).unwrap().reason,
                expected
            );
        }
    }

    #[test]
    fn rejects_truncated_win_messages() {
        let bytes = [3, 0, 0, 0, 5, 0, 2];
        for length in 0..bytes.len() {
            assert!(CoreMessage::try_from(&bytes[..length]).is_err());
        }
    }
}
