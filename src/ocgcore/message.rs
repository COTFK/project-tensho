use anyhow::anyhow;

use super::actions::ActiveCard;
use super::actions::AvailableActions;
use super::constants::BattlePosition;
use super::constants::CardController;
use super::constants::CardLocation;
use super::messages::SelectCardMessage;
use super::messages::SelectUnselectMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreMessage {
    Retry,
    Idle(AvailableActions),
    SelectEffectYN(ActiveCard),
    SelectYesNo {
        player: u8,
        card_code: u32,
        string_index: usize,
    },
    SelectCard(SelectCardMessage),
    SelectChain(Vec<ActiveCard>),
    SelectPlace(Vec<(CardLocation, u8)>),
    SelectPosition(Vec<BattlePosition>),
    SelectUnselectCard(SelectUnselectMessage),
}

impl TryFrom<Vec<u8>> for CoreMessage {
    type Error = anyhow::Error;

    fn try_from(messages: Vec<u8>) -> anyhow::Result<Self, Self::Error> {
        let msg_value = messages
            .get(4)
            .ok_or(anyhow!("Unable to get message value"))?;

        tracing::debug!("Received message {msg_value:?} with bytes: {messages:?}");

        match msg_value {
            1 => Ok(CoreMessage::Retry),
            11 => {
                let actions = AvailableActions::try_from(&messages[..])?;

                Ok(CoreMessage::Idle(actions))
            }
            12 => {
                let player = messages[5];
                let card_code =
                    u32::from_le_bytes([messages[6], messages[7], messages[8], messages[9]]);
                let location = CardLocation::try_from(messages[11]).unwrap();
                let sequence = messages[12];

                Ok(CoreMessage::SelectEffectYN(ActiveCard {
                    card_code,
                    controller: CardController::Player,
                    location,
                    position: None,
                    sequence,
                    chain_option: None,
                    description: None,
                    is_selected: false,
                }))
            }
            13 => {
                let player = messages[5];

                let description_id = u64::from_le_bytes([
                    messages[6],
                    messages[7],
                    messages[8],
                    messages[9],
                    messages[10],
                    messages[11],
                    messages[12],
                    messages[13],
                ]);

                let card_code = ((description_id >> 20) & 0xffff_ffff) as u32;
                let string_index = (description_id & 0xfffff) as usize;

                Ok(CoreMessage::SelectYesNo {
                    player,
                    card_code,
                    string_index,
                })
            }
            15 => Ok(CoreMessage::SelectCard(SelectCardMessage::try_from(
                &messages[..],
            )?)),
            16 => {
                let count =
                    u32::from_le_bytes([messages[16], messages[17], messages[18], messages[19]])
                        as usize;
                let mut chainables = Vec::new();

                let mut offset = 20;
                for chain_option in 0..count {
                    if offset + 7 <= messages.len() {
                        let card_code = u32::from_le_bytes([
                            messages[offset],
                            messages[offset + 1],
                            messages[offset + 2],
                            messages[offset + 3],
                        ]);

                        let location_byte = messages
                            .get(offset + 5)
                            .ok_or(anyhow!("Failed to get location byte"))?;
                        let sequence = messages
                            .get(offset + 6)
                            .ok_or(anyhow!("Failed to get sequence byte"))?;
                        let controller = CardController::try_from(messages[offset + 4])?;
                        let location = CardLocation::try_from(*location_byte)?;

                        chainables.push(ActiveCard {
                            card_code,
                            controller,
                            location,
                            position: None,
                            sequence: *sequence,
                            chain_option: Some(chain_option as u8),
                            description: None,
                            is_selected: false,
                        })
                    }

                    // Advance by 23 bytes to cleanly clear the trailing descriptive payload fields
                    offset += 23;
                }

                Ok(CoreMessage::SelectChain(chainables))
            }
            18 => {
                let mut zones = Vec::new();

                let _player = messages[5];
                let _count = messages[6]; // Number of places to pick

                // Extract the 32-bit zone layout mask
                let zone_mask =
                    u32::from_le_bytes([messages[7], messages[8], messages[9], messages[10]]);

                // 1. Check Main Monster Zones (Bits 0 to 4)
                // In ocgcore, a bit value of 0 means the zone is AVAILABLE
                for seq in 0..5 {
                    if (zone_mask & (1 << seq)) == 0 {
                        zones.push((CardLocation::MonsterZone, seq));
                    }
                }

                // 2. Check Extra Monster Zones (Bits 5 and 6)
                // Note: In the response layout, EMZONE is submitted as LOCATION_MZONE
                // with index 5 (Left EMZ) or index 6 (Right EMZ).
                for seq in 5..7 {
                    if (zone_mask & (1 << seq)) == 0 {
                        zones.push((CardLocation::MonsterZone, seq));
                    }
                }

                // 3. Check Spell & Trap Zones (Bits 8 to 12)
                // Bit 8 is Column 1 (shift 8), Bit 12 is Column 5 (shift 12)
                for seq in 0..5 {
                    if (zone_mask & (1 << (8 + seq))) == 0 {
                        zones.push((CardLocation::SpellTrapZone, seq));
                    }
                }

                // 4. Check Field Spell Zone (Bit 13)
                if (zone_mask & (1 << 13)) == 0 {
                    zones.push((CardLocation::SpellTrapZone, 5)); // Field zone index index
                }

                Ok(CoreMessage::SelectPlace(zones))
            }
            19 => {
                let mut allowed_positions = Vec::new();
                let position_mask = messages[10];

                if (position_mask & 0x1) != 0 {
                    allowed_positions.push(BattlePosition::FaceUpAttack);
                }
                if (position_mask & 0x2) != 0 {
                    allowed_positions.push(BattlePosition::FaceDownAttack);
                }
                if (position_mask & 0x4) != 0 {
                    allowed_positions.push(BattlePosition::FaceUpDefense);
                }
                if (position_mask & 0x8) != 0 {
                    allowed_positions.push(BattlePosition::FaceDownDefense);
                }

                Ok(CoreMessage::SelectPosition(allowed_positions))
            }
            26 => Ok(CoreMessage::SelectUnselectCard(
                SelectUnselectMessage::try_from(&messages[..])?,
            )),
            _ => anyhow::bail!("Received wrong message: {msg_value}"),
        }
    }
}
