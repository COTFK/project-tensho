use anyhow::anyhow;

use super::actions::ActiveCard;
use super::actions::AvailableActions;
use super::constants::CardController;
use super::constants::CardLocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreMessage {
    Retry,
    Idle(AvailableActions),
    SelectEffectYN(ActiveCard),
    SelectCard,
    SelectChain(Vec<ActiveCard>),
    SelectPlace,
}

impl TryFrom<Vec<u8>> for CoreMessage {
    type Error = anyhow::Error;

    fn try_from(messages: Vec<u8>) -> anyhow::Result<Self, Self::Error> {
        let msg_value = messages
            .get(4)
            .ok_or(anyhow!("Unable to get message value"))?;

        tracing::debug!("Received message {msg_value} with bytes: {messages:?}");

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

                tracing::debug!(
                    "SelectEffectYN -> Player: {}, Card: {}, Location: {:?}, Zone Index: {}",
                    player, card_code, location, sequence
                );

                Ok(CoreMessage::SelectEffectYN(
                    ActiveCard { card_code, controller: CardController::Player, location, sequence, chain_option: None }
                ))
            },
            15 => Ok(CoreMessage::SelectCard),
            16 => {
                let count =
                    u32::from_le_bytes([messages[16], messages[17], messages[18], messages[19]])
                        as usize;
                let mut chainables = Vec::new();

                tracing::debug!("SelectChain -> Count: {}", count);

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

                        tracing::debug!(
                            "  Option #{}: Card ID {}, Controller: {:?}, Location: {:?}, Slot: {}",
                            chain_option,
                            card_code,
                            controller,
                            location,
                            sequence
                        );

                        chainables.push(ActiveCard {
                            card_code,
                            controller,
                            location,
                            sequence: *sequence,
                            chain_option: Some(chain_option as u8),
                        })
                    }

                    // Advance by 23 bytes to cleanly clear the trailing descriptive payload fields
                    offset += 23;
                }

                Ok(CoreMessage::SelectChain(chainables))
            }
            18 => Ok(CoreMessage::SelectPlace),
            _ => anyhow::bail!("Received wrong message: {msg_value}"),
        }
    }
}
