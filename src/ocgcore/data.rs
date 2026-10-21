use bitflags::bitflags;

use super::constants::BattlePosition;
use super::constants::CardController;
use super::constants::CardLocation;

#[derive(Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct HandCard {
    pub index: usize,
    pub code: u32,
    pub is_activatable_or_chainable: bool,
    pub activate_index: Option<u8>,
    pub chain_index: Option<u8>,
    pub normal_summon_index: Option<u8>,
    pub monster_set_index: Option<u8>,
    pub spell_trap_set_index: Option<u8>,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Zone {
    pub location: CardLocation,
    pub sequence: u8,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct CardData {
    pub action_index: Option<u8>,
    pub card_code: u32,
    pub controller: CardController,
    pub location: CardLocation,
    pub position: Option<BattlePosition>,
    pub sequence: u8,
    pub description: Option<u32>,
    pub is_selected: bool,
    pub level: Option<u32>,
    pub rank: Option<u32>,
    pub attack: Option<u32>,
    pub defense: Option<u32>,
    pub card_type: CardType,
    pub link_rating: Option<u32>,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct CardType: u32 {
        const MONSTER = ocgcore_ffi::constants::TYPE_MONSTER;
        const SPELL = ocgcore_ffi::constants::TYPE_SPELL;
        const TRAP = ocgcore_ffi::constants::TYPE_TRAP;
        const NORMAL = ocgcore_ffi::constants::TYPE_NORMAL;
        const EFFECT = ocgcore_ffi::constants::TYPE_EFFECT;
        const FUSION = ocgcore_ffi::constants::TYPE_FUSION;
        const RITUAL = ocgcore_ffi::constants::TYPE_RITUAL;
        const TRAPMONSTER = ocgcore_ffi::constants::TYPE_TRAPMONSTER;
        const SPIRIT = ocgcore_ffi::constants::TYPE_SPIRIT;
        const UNION = ocgcore_ffi::constants::TYPE_UNION;
        const GEMINI = ocgcore_ffi::constants::TYPE_GEMINI;
        const TUNER = ocgcore_ffi::constants::TYPE_TUNER;
        const SYNCHRO = ocgcore_ffi::constants::TYPE_SYNCHRO;
        const TOKEN = ocgcore_ffi::constants::TYPE_TOKEN;
        const MAXIMUM = ocgcore_ffi::constants::TYPE_MAXIMUM;
        const QUICKPLAY = ocgcore_ffi::constants::TYPE_QUICKPLAY;
        const CONTINUOUS = ocgcore_ffi::constants::TYPE_CONTINUOUS;
        const EQUIP = ocgcore_ffi::constants::TYPE_EQUIP;
        const FIELD = ocgcore_ffi::constants::TYPE_FIELD;
        const COUNTER = ocgcore_ffi::constants::TYPE_COUNTER;
        const FLIP = ocgcore_ffi::constants::TYPE_FLIP;
        const TOON = ocgcore_ffi::constants::TYPE_TOON;
        const XYZ = ocgcore_ffi::constants::TYPE_XYZ;
        const PENDULUM = ocgcore_ffi::constants::TYPE_PENDULUM;
        const SPSUMMON = ocgcore_ffi::constants::TYPE_SPSUMMON;
        const LINK = ocgcore_ffi::constants::TYPE_LINK;
    }
}
