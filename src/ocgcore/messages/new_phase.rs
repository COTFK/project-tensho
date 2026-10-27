use crate::ocgcore::constants::Phase;
use crate::ocgcore::utility::{read_u16, read_u32};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NewPhaseMessageData {
    pub phase: Phase,
}

impl TryFrom<&[u8]> for NewPhaseMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 0;
        let length = read_u32(bytes, &mut cursor, "message length")?;
        anyhow::ensure!(length == 3, "Invalid new phase message length: {length}");
        cursor += 1;
        let phase = Phase::try_from(read_u16(bytes, &mut cursor, "phase")?)?;
        Ok(Self { phase })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocgcore::messages::CoreMessage;

    #[test]
    fn parses_all_phases() {
        for (value, phase) in [
            (0x01u16, Phase::Draw),
            (0x02, Phase::Standby),
            (0x04, Phase::Main1),
            (0x08, Phase::BattleStart),
            (0x10, Phase::BattleStep),
            (0x20, Phase::Damage),
            (0x40, Phase::DamageCalculation),
            (0x80, Phase::Battle),
            (0x100, Phase::Main2),
            (0x200, Phase::End),
        ] {
            let [low, high] = value.to_le_bytes();
            let bytes = [3, 0, 0, 0, 41, low, high];
            assert_eq!(
                CoreMessage::try_from(bytes.as_slice()).unwrap(),
                CoreMessage::NewPhase(NewPhaseMessageData { phase })
            );
        }
    }

    #[test]
    fn ignores_following_framed_message() {
        let bytes = [
            3, 0, 0, 0, 41, 1, 0, 11, 0, 0, 0, 2, 1, 0, 27, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(
            CoreMessage::try_from(bytes.as_slice()).unwrap(),
            CoreMessage::NewPhase(NewPhaseMessageData { phase: Phase::Draw })
        );
    }

    #[test]
    fn rejects_truncated_or_invalid_messages() {
        let bytes = [3, 0, 0, 0, 41, 1, 0];
        for length in 0..bytes.len() {
            assert!(CoreMessage::try_from(&bytes[..length]).is_err());
        }
        assert!(CoreMessage::try_from([3, 0, 0, 0, 41, 3, 0].as_slice()).is_err());
        assert!(CoreMessage::try_from([2, 0, 0, 0, 41, 1, 0].as_slice()).is_err());
    }
}
