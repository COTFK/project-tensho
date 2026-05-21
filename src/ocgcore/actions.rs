use anyhow::Context;
use std::collections::HashMap;

use super::constants::CardController;
use super::constants::CardLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveCard {
    pub card_code: u32,
    pub controller: CardController,
    pub location: CardLocation,
    pub sequence: u8,
    pub chain_option: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AvailableActions {
    pub playerid: u8,
    pub normal_summons: Vec<ActiveCard>,   // Block 1: Summonable
    pub special_summons: Vec<ActiveCard>,  // Block 2: SpSummonable
    pub battle_positions: Vec<ActiveCard>, // Block 3: Repositionable
    pub monster_sets: Vec<ActiveCard>,     // Block 4: MSetable (Monster Set)
    pub spell_trap_sets: Vec<ActiveCard>,  // Block 5: Setable (S/T Set)
    pub activatable_effects: Vec<ActiveCard>, // Block 6: Activatable
    pub can_to_bp: bool,
    pub can_to_ep: bool,
    pub can_shuffle: bool,
}

impl AvailableActions {
    pub fn get_normal_summons(&self) -> HashMap<u8, u16> {
        self.normal_summons
            .iter()
            .enumerate()
            .map(|(index, card)| (card.sequence, index as u16))
            .collect()
    }

    pub fn get_activatable_effects(&self) -> HashMap<u16, ActiveCard> {
        self.activatable_effects
            .iter()
            .enumerate()
            .map(|(index, card)| (index as u16, card.clone()))
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

        fn read_u8(raw_bytes: &[u8], cursor: &mut usize, label: &str) -> anyhow::Result<u8> {
            let value = raw_bytes
                .get(*cursor)
                .copied()
                .with_context(|| format!("Missing {label} at offset {}", *cursor))?;
            *cursor += 1;
            Ok(value)
        }

        fn read_u32(raw_bytes: &[u8], cursor: &mut usize, label: &str) -> anyhow::Result<u32> {
            let bytes = raw_bytes
                .get(*cursor..*cursor + 4)
                .with_context(|| format!("Missing {label} at offset {}", *cursor))?;
            *cursor += 4;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        }

        fn read_u64(raw_bytes: &[u8], cursor: &mut usize, label: &str) -> anyhow::Result<u64> {
            let bytes = raw_bytes
                .get(*cursor..*cursor + 8)
                .with_context(|| format!("Missing {label} at offset {}", *cursor))?;
            *cursor += 8;
            Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
        }

        fn read_block_common(
            raw_bytes: &[u8],
            cursor: &mut usize,
            block_name: &'static str,
            item_reader: impl Fn(&[u8], &mut usize) -> anyhow::Result<ActiveCard>,
        ) -> anyhow::Result<Vec<ActiveCard>> {
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
            for _ in 0..count {
                items.push(item_reader(raw_bytes, cursor)?);
            }

            Ok(items)
        }

        let mut cursor = message_offset + 1;
        let playerid = read_u8(raw_bytes, &mut cursor, "playerid")?;

        let normal_summons =
            read_block_common(raw_bytes, &mut cursor, "normal_summons", |raw, c| {
                Ok(ActiveCard {
                    card_code: read_u32(raw, c, "normal_summons.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "normal_summons.controller",
                    )?)?,
                    location: CardLocation::try_from(read_u8(raw, c, "normal_summons.location")?)?,
                    sequence: read_u32(raw, c, "normal_summons.sequence")? as u8,
                    chain_option: None,
                })
            })?;

        let special_summons =
            read_block_common(raw_bytes, &mut cursor, "special_summons", |raw, c| {
                Ok(ActiveCard {
                    card_code: read_u32(raw, c, "special_summons.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "special_summons.controller",
                    )?)?,
                    location: CardLocation::try_from(read_u8(raw, c, "special_summons.location")?)?,
                    sequence: read_u32(raw, c, "special_summons.sequence")? as u8,
                    chain_option: None,
                })
            })?;

        let battle_positions =
            read_block_common(raw_bytes, &mut cursor, "battle_positions", |raw, c| {
                Ok(ActiveCard {
                    card_code: read_u32(raw, c, "battle_positions.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "battle_positions.controller",
                    )?)?,
                    location: CardLocation::try_from(read_u8(
                        raw,
                        c,
                        "battle_positions.location",
                    )?)?,
                    sequence: read_u8(raw, c, "battle_positions.sequence")?,
                    chain_option: None,
                })
            })?;

        let monster_sets = read_block_common(raw_bytes, &mut cursor, "monster_sets", |raw, c| {
            Ok(ActiveCard {
                card_code: read_u32(raw, c, "monster_sets.card_code")?,
                controller: CardController::try_from(read_u8(raw, c, "monster_sets.controller")?)?,
                location: CardLocation::try_from(read_u8(raw, c, "monster_sets.location")?)?,
                sequence: read_u32(raw, c, "monster_sets.sequence")? as u8,
                chain_option: None,
            })
        })?;

        let spell_trap_sets =
            read_block_common(raw_bytes, &mut cursor, "spell_trap_sets", |raw, c| {
                Ok(ActiveCard {
                    card_code: read_u32(raw, c, "spell_trap_sets.card_code")?,
                    controller: CardController::try_from(read_u8(
                        raw,
                        c,
                        "spell_trap_sets.controller",
                    )?)?,
                    location: CardLocation::try_from(read_u8(raw, c, "spell_trap_sets.location")?)?,
                    sequence: read_u32(raw, c, "spell_trap_sets.sequence")? as u8,
                    chain_option: None,
                })
            })?;

        let activatable_effects =
            read_block_common(raw_bytes, &mut cursor, "activatable_effects", |raw, c| {
                let card_code = read_u32(raw, c, "activatable_effects.card_code")?;
                let controller = read_u8(raw, c, "activatable_effects.controller")?;
                let location = read_u8(raw, c, "activatable_effects.location")?;
                let sequence = read_u32(raw, c, "activatable_effects.sequence")? as u8;
                let _description = read_u64(raw, c, "activatable_effects.description")?;
                let _client_mode = read_u8(raw, c, "activatable_effects.client_mode")?;

                Ok(ActiveCard {
                    card_code,
                    controller: CardController::try_from(controller)?,
                    location: CardLocation::try_from(location)?,
                    sequence,
                    chain_option: None,
                })
            })?;

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
