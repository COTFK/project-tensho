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
    pub attack: Option<u32>,
    pub defense: Option<u32>,
    pub card_type: CardType,
    pub link_rating: Option<u32>,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct CardType: u32 {
        const MONSTER       = 0x1;
        const SPELL         = 0x2;
        const TRAP          = 0x4;
        const NORMAL        = 0x10;
        const EFFECT        = 0x20;
        const FUSION        = 0x40;
        const RITUAL        = 0x80;
        const TRAPMONSTER   = 0x100;
        const SPIRIT        = 0x200;
        const UNION         = 0x400;
        const GEMINI        = 0x800;
        const TUNER         = 0x1000;
        const SYNCHRO       = 0x2000;
        const TOKEN         = 0x4000;
        const MAXIMUM       = 0x8000;
        const QUICKPLAY     = 0x10000;
        const CONTINUOUS    = 0x20000;
        const EQUIP         = 0x40000;
        const FIELD         = 0x80000;
        const COUNTER       = 0x100000;
        const FLIP          = 0x200000;
        const TOON          = 0x400000;
        const XYZ           = 0x800000;
        const PENDULUM      = 0x1000000;
        const SPSUMMON      = 0x2000000;
        const LINK          = 0x4000000;
    }
}
