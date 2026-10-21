use crate::ocgcore::constants::BattlePosition;
use ocgcore_ffi::constants::{
    POS_FACEDOWN_ATTACK, POS_FACEDOWN_DEFENSE, POS_FACEUP_ATTACK, POS_FACEUP_DEFENSE,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
        let position_mask = u32::from(bytes[10]);

        if (position_mask & POS_FACEUP_ATTACK) != 0 {
            positions.push(BattlePosition::FaceUpAttack);
        }
        if (position_mask & POS_FACEDOWN_ATTACK) != 0 {
            positions.push(BattlePosition::FaceDownAttack);
        }
        if (position_mask & POS_FACEUP_DEFENSE) != 0 {
            positions.push(BattlePosition::FaceUpDefense);
        }
        if (position_mask & POS_FACEDOWN_DEFENSE) != 0 {
            positions.push(BattlePosition::FaceDownDefense);
        }

        Ok(SelectPositionMessageData {
            player,
            card_code,
            positions,
        })
    }
}
