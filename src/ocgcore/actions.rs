use std::collections::HashMap;

use super::constants::BattlePosition;
use super::constants::CardController;
use super::constants::CardLocation;
use crate::ocgcore::utility::read_u8;
use crate::ocgcore::utility::read_u32;
use crate::ocgcore::utility::read_u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardData {
    pub action_index: Option<u8>,
    pub card_code: u32,
    pub controller: CardController,
    pub location: CardLocation,
    pub position: Option<BattlePosition>,
    pub sequence: u8,
    pub description: Option<u32>,
    pub is_selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AvailableActions {
    pub playerid: u8,
    pub normal_summons: Vec<CardData>,      // Block 1: Summonable
    pub special_summons: Vec<CardData>,     // Block 2: SpSummonable
    pub battle_positions: Vec<CardData>,    // Block 3: Repositionable
    pub monster_sets: Vec<CardData>,        // Block 4: MSetable (Monster Set)
    pub spell_trap_sets: Vec<CardData>,     // Block 5: Setable (S/T Set)
    pub activatable_effects: Vec<CardData>, // Block 6: Activatable
    pub can_to_bp: bool,
    pub can_to_ep: bool,
    pub can_shuffle: bool,
}

impl AvailableActions {
    pub fn get_normal_summons(&self) -> &Vec<CardData> {
        &self.normal_summons
    }

    pub fn get_special_summons(&self) -> &Vec<CardData> {
        &self.special_summons
    }

    pub fn get_activatable_effects(&self) -> HashMap<u16, CardData> {
        self.activatable_effects
            .iter()
            .enumerate()
            .map(|(index, card)| (index as u16, *card))
            .collect()
    }
}

impl TryFrom<&[u8]> for AvailableActions {
    type Error = anyhow::Error;

    fn try_from(raw_bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        if raw_bytes.len() < 2 {
            anyhow::bail!(
                "Buffer too short to contain a valid payload (len: {})",
                raw_bytes.len()
            );
        }

        let message_offset = if raw_bytes[0] == 11 {
            0
        } else if raw_bytes.len() >= 5 && raw_bytes[4] == 11 {
            let declared_len = u32::from_le_bytes(raw_bytes[0..4].try_into().unwrap()) as usize;
            if declared_len != raw_bytes.len().saturating_sub(4) {
                anyhow::bail!(
                    "Length prefix mismatch: declared {} but buffer has {} payload bytes",
                    declared_len,
                    raw_bytes.len().saturating_sub(4)
                );
            }

            4
        } else {
            0
        };

        if raw_bytes.get(message_offset).copied() != Some(11) {
            anyhow::bail!(
                "Invalid message ID: Expected MSG_SELECT_IDLECMD (11), got {}",
                raw_bytes.get(message_offset).copied().unwrap_or_default()
            );
        }

        fn read_block_common(
            raw_bytes: &[u8],
            cursor: &mut usize,
            block_name: &'static str,
            item_reader: impl Fn(&[u8], &mut usize, usize) -> anyhow::Result<CardData>,
        ) -> anyhow::Result<Vec<CardData>> {
            let count = read_u32(raw_bytes, cursor, block_name)? as usize;

            if count > 50 {
                anyhow::bail!(
                    "Sanity check failed parsing {} at offset {}. Parsed impossible count {}",
                    block_name,
                    *cursor - 4,
                    count
                );
            }

            let mut items = Vec::with_capacity(count);
            for index in 0..count {
                items.push(item_reader(raw_bytes, cursor, index)?);
            }

            Ok(items)
        }

        let mut cursor = message_offset + 1;
        let playerid = read_u8(raw_bytes, &mut cursor, "playerid")?;

        let normal_summons =
            read_block_common(raw_bytes, &mut cursor, "normal_summons", |raw, c, idx| {
                Ok(CardData {
                    card_code: read_u32(raw, c, "normal_summons.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "normal_summons.controller",
                    )?)?,
                    position: None,
                    location: CardLocation::try_from(read_u8(raw, c, "normal_summons.location")?)?,
                    sequence: read_u32(raw, c, "normal_summons.index")? as u8,
                    action_index: Some(idx as u8),
                    description: None,
                    is_selected: false,
                })
            })?;

        let special_summons =
            read_block_common(raw_bytes, &mut cursor, "special_summons", |raw, c, idx| {
                Ok(CardData {
                    card_code: read_u32(raw, c, "special_summons.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "special_summons.controller",
                    )?)?,
                    position: None,
                    location: CardLocation::try_from(read_u8(raw, c, "special_summons.location")?)?,
                    sequence: read_u32(raw, c, "special_summons.index")? as u8,
                    action_index: Some(idx as u8),
                    description: None,
                    is_selected: false,
                })
            })?;

        let battle_positions =
            read_block_common(raw_bytes, &mut cursor, "battle_positions", |raw, c, idx| {
                Ok(CardData {
                    card_code: read_u32(raw, c, "battle_positions.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "battle_positions.controller",
                    )?)?,
                    position: None,
                    location: CardLocation::try_from(read_u8(
                        raw,
                        c,
                        "battle_positions.location",
                    )?)?,
                    sequence: read_u8(raw, c, "battle_positions.index")?,
                    action_index: Some(idx as u8),
                    description: None,
                    is_selected: false,
                })
            })?;

        let monster_sets =
            read_block_common(raw_bytes, &mut cursor, "monster_sets", |raw, c, idx| {
                Ok(CardData {
                    card_code: read_u32(raw, c, "monster_sets.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "monster_sets.controller",
                    )?)?,
                    location: CardLocation::try_from(read_u8(raw, c, "monster_sets.location")?)?,
                    position: None,
                    sequence: read_u32(raw, c, "monster_sets.index")? as u8,
                    action_index: Some(idx as u8),
                    description: None,
                    is_selected: false,
                })
            })?;

        let spell_trap_sets =
            read_block_common(raw_bytes, &mut cursor, "spell_trap_sets", |raw, c, idx| {
                Ok(CardData {
                    card_code: read_u32(raw, c, "spell_trap_sets.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "spell_trap_sets.controller",
                    )?)?,
                    position: None,
                    location: CardLocation::try_from(read_u8(raw, c, "spell_trap_sets.location")?)?,
                    sequence: read_u32(raw, c, "spell_trap_sets.index")? as u8,
                    action_index: Some(idx as u8),
                    description: None,
                    is_selected: false,
                })
            })?;

        let activatable_effects = read_block_common(
            raw_bytes,
            &mut cursor,
            "activatable_effects",
            |raw, c, idx| {
                let card_code = read_u32(raw, c, "activatable_effects.card_code")?;
                let controller = read_u8(raw, c, "activatable_effects.controller")?;
                let location = read_u8(raw, c, "activatable_effects.location")?;
                let sequence = read_u32(raw, c, "activatable_effects.sequence")? as u8;
                let _description = read_u64(raw, c, "activatable_effects.description")?;
                let _client_mode = read_u8(raw, c, "activatable_effects.client_mode")?;

                let description_id = (_description & 0xFFFFF) as u32;

                Ok(CardData {
                    card_code,
                    controller: CardController::try_from(controller)?,
                    location: CardLocation::try_from(location)?,
                    position: None,
                    sequence,
                    action_index: Some(idx as u8),
                    description: Some(description_id),
                    is_selected: false,
                })
            },
        )?;

        let can_to_bp = read_u8(raw_bytes, &mut cursor, "can_to_bp")? != 0;
        let can_to_ep = read_u8(raw_bytes, &mut cursor, "can_to_ep")? != 0;
        let can_shuffle = read_u8(raw_bytes, &mut cursor, "can_shuffle")? != 0;

        Ok(Self {
            playerid,
            normal_summons,
            special_summons,
            battle_positions,
            monster_sets,
            spell_trap_sets,
            activatable_effects,
            can_to_bp,
            can_to_ep,
            can_shuffle,
        })
    }
}
