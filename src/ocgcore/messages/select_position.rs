use crate::ocgcore::constants::BattlePosition;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectPositionMessageData {
    pub player: u8,
    pub card_code: u32,
    pub positions: Vec<BattlePosition>,
}

impl TryFrom<&[u8]> for SelectPositionMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let mut positions = Vec::new();
        let player = bytes[5];
        let card_code = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        let position_mask = bytes[10];

        if (position_mask & 0x1) != 0 {
            positions.push(BattlePosition::FaceUpAttack);
        }
        if (position_mask & 0x2) != 0 {
            positions.push(BattlePosition::FaceDownAttack);
        }
        if (position_mask & 0x4) != 0 {
            positions.push(BattlePosition::FaceUpDefense);
        }
        if (position_mask & 0x8) != 0 {
            positions.push(BattlePosition::FaceDownDefense);
        }

        Ok(SelectPositionMessageData {
            player,
            card_code,
            positions,
        })
    }
}
