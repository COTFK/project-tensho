use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::data::CardData;
use crate::ocgcore::utility::{read_u8, read_u32, read_u64};

#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct IdleMessageData {
    pub playerid: u8,
    pub normal_summons: Vec<CardData>,
    pub special_summons: Vec<CardData>,
    pub monster_sets: Vec<CardData>,
    pub spell_trap_sets: Vec<CardData>,
    pub activatable_effects: Vec<CardData>,
}

fn read_cards(bytes: &[u8], cursor: &mut usize, effects: bool) -> anyhow::Result<Vec<CardData>> {
    let count = read_u32(bytes, cursor, "idle card count")? as usize;
    let width = if effects { 19 } else { 10 };
    anyhow::ensure!(
        count <= (bytes.len() - *cursor) / width,
        "Truncated idle cards"
    );
    let mut cards = Vec::with_capacity(count);
    for index in 0..count {
        let card_code = read_u32(bytes, cursor, "card code")?;
        let controller = Player::try_from(read_u8(bytes, cursor, "controller")?)?;
        let location = CardLocation::try_from(read_u8(bytes, cursor, "location")?)?;
        let sequence = read_u32(bytes, cursor, "sequence")? as u8;
        let description = if effects {
            let description = read_u64(bytes, cursor, "description")?;
            read_u8(bytes, cursor, "client mode")?;
            Some((description & 0xFFFFF) as u32)
        } else {
            None
        };
        cards.push(CardData {
            card_code,
            controller,
            location,
            sequence,
            action_index: Some(index as u8),
            description,
            ..Default::default()
        });
    }
    Ok(cards)
}

impl TryFrom<&[u8]> for IdleMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = 5;
        let playerid = read_u8(bytes, &mut cursor, "playerid")?;
        let normal_summons = read_cards(bytes, &mut cursor, false)?;
        let special_summons = read_cards(bytes, &mut cursor, false)?;
        let count = read_u32(bytes, &mut cursor, "position count")? as usize;
        anyhow::ensure!(
            count <= (bytes.len() - cursor) / 7,
            "Truncated position cards"
        );
        cursor += count * 7;
        let monster_sets = read_cards(bytes, &mut cursor, false)?;
        let spell_trap_sets = read_cards(bytes, &mut cursor, false)?;
        let activatable_effects = read_cards(bytes, &mut cursor, true)?;
        read_u8(bytes, &mut cursor, "can enter battle phase")?;
        read_u8(bytes, &mut cursor, "can enter end phase")?;
        read_u8(bytes, &mut cursor, "can shuffle")?;
        Ok(Self {
            playerid,
            normal_summons,
            special_summons,
            monster_sets,
            spell_trap_sets,
            activatable_effects,
        })
    }
}
