use ocgcore_ffi::constants::{
    PHASE_BATTLE, PHASE_BATTLE_START, PHASE_BATTLE_STEP, PHASE_DAMAGE, PHASE_DAMAGE_CAL,
    PHASE_DRAW, PHASE_END, PHASE_MAIN1, PHASE_MAIN2, PHASE_STANDBY,
};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    Draw,
    Standby,
    Main1,
    BattleStart,
    BattleStep,
    Damage,
    DamageCalculation,
    Battle,
    Main2,
    End,
}

impl TryFrom<u16> for Phase {
    type Error = anyhow::Error;

    fn try_from(value: u16) -> anyhow::Result<Self> {
        match u32::from(value) {
            PHASE_DRAW => Ok(Self::Draw),
            PHASE_STANDBY => Ok(Self::Standby),
            PHASE_MAIN1 => Ok(Self::Main1),
            PHASE_BATTLE_START => Ok(Self::BattleStart),
            PHASE_BATTLE_STEP => Ok(Self::BattleStep),
            PHASE_DAMAGE => Ok(Self::Damage),
            PHASE_DAMAGE_CAL => Ok(Self::DamageCalculation),
            PHASE_BATTLE => Ok(Self::Battle),
            PHASE_MAIN2 => Ok(Self::Main2),
            PHASE_END => Ok(Self::End),
            _ => anyhow::bail!("Received invalid phase value: {value}"),
        }
    }
}
