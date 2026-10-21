use std::fmt::Display;

use ocgcore_ffi::constants::{
    LOCATION_DECK, LOCATION_EXTRA, LOCATION_GRAVE, LOCATION_HAND, LOCATION_MZONE, LOCATION_REMOVED,
    LOCATION_SZONE,
};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CardLocation {
    #[default]
    Deck = LOCATION_DECK as isize,
    Hand = LOCATION_HAND as isize,
    MonsterZone = LOCATION_MZONE as isize,
    SpellTrapZone = LOCATION_SZONE as isize,
    Graveyard = LOCATION_GRAVE as isize,
    Banishment = LOCATION_REMOVED as isize,
    ExtraDeck = LOCATION_EXTRA as isize,
}

impl TryFrom<u8> for CardLocation {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match u32::from(value) {
            LOCATION_DECK => Ok(CardLocation::Deck),
            LOCATION_HAND => Ok(CardLocation::Hand),
            LOCATION_MZONE => Ok(CardLocation::MonsterZone),
            LOCATION_SZONE => Ok(CardLocation::SpellTrapZone),
            LOCATION_GRAVE => Ok(CardLocation::Graveyard),
            LOCATION_REMOVED => Ok(CardLocation::Banishment),
            LOCATION_EXTRA => Ok(CardLocation::ExtraDeck),
            _ => anyhow::bail!("Received wrong location value: {value}"),
        }
    }
}

impl Display for CardLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            CardLocation::Deck => "Deck",
            CardLocation::Hand => "Hand",
            CardLocation::ExtraDeck => "Extra Deck",
            CardLocation::Graveyard => "GY",
            CardLocation::Banishment => "Banishment",
            CardLocation::MonsterZone => "Monster Zone",
            CardLocation::SpellTrapZone => "Spell/Trap Zone",
        };

        write!(f, "{}", text)
    }
}
