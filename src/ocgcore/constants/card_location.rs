use std::fmt::Display;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardLocation {
    Deck = 1,
    Hand = 2,
    MonsterZone = 4,
    SpellTrapZone = 8,
    Graveyard = 16,
    Banishment = 32,
    ExtraDeck = 64,
}

impl TryFrom<u8> for CardLocation {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self, Self::Error> {
        match value {
            1 => Ok(CardLocation::Deck),
            2 => Ok(CardLocation::Hand),
            4 => Ok(CardLocation::MonsterZone),
            8 => Ok(CardLocation::SpellTrapZone),
            16 => Ok(CardLocation::Graveyard),
            32 => Ok(CardLocation::Banishment),
            64 => Ok(CardLocation::ExtraDeck),
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
