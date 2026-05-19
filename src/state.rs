use dioxus::prelude::*;
use std::collections::HashMap;

use crate::ocgcore::Duel;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::CoreMessage;
use crate::ocgcore::ActiveCard;

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedCard {
    pub location: CardLocation,
    pub index: u8
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuelState {
    pub duel: Duel,
    pub hand_contents: Signal<Vec<u32>>,
    pub selected_card: Signal<Option<SelectedCard>>,
    pub normal_summons: Signal<HashMap<u8, u16>>,
    pub waiting_on_input: Signal<bool>,
    pub monsters: Signal<Vec<u32>>,
    pub card_prompting_to_activate: Signal<Vec<ActiveCard>>,
}

pub fn send_user_response(response: UserResponse) {
    let mut state = use_context::<DuelState>();

    let duel = state.duel;

    debug!("{:?} requested", response);
    duel.set_response(response);

    // Clean up state and wait for new data
    state.waiting_on_input.set(false);
    state.selected_card.set(None);
    state.normal_summons.clear();
    state.card_prompting_to_activate.clear();
}

pub fn handle_core_message() {
    let state = use_context::<DuelState>();

    let duel = state.duel;

    let mut normal_summons = state.normal_summons;
    let mut waiting_on_input = state.waiting_on_input;
    let mut card_prompting_to_activate = state.card_prompting_to_activate;
    let mut monsters = state.monsters;
    let mut hand_contents = state.hand_contents;

    let messages = duel.get_messages();
    let msg_byte = messages.get(4).unwrap();
    let msg_type = CoreMessage::try_from(*msg_byte).unwrap();

    debug!("Current message is {msg_type:?} with bytes {messages:?}");

    match msg_type {
        CoreMessage::Retry => {
            debug!("Received Retry - this shouldn't happen.");
        }
        CoreMessage::Idle => {
            let actions = duel.get_available_actions(messages).unwrap();

            normal_summons.set(actions.get_normal_summons());
        }
        CoreMessage::SelectPlace => send_user_response(UserResponse::Place {
            controller: CardController::Player as u8,
            location: CardLocation::MonsterZone as u8,
            index: 0,
        }),
        CoreMessage::SelectChain => {
            if messages.len() < 20 {
                return;
            }

            let player = messages[5];
            let count: usize;

            if messages.len() == 20 {
                count = 0;
            } else {
                count = u32::from_le_bytes([messages[16], messages[17], messages[18], messages[19]])
                    as usize;
            }

            debug!("SelectChain -> Player: {}, Count: {}", player, count);

            if count == 0 {
                debug!("No selectable chain choices present. Declining/Passing priority.");

                send_user_response(UserResponse::PassPriority);
            } else {
                debug!(
                    "Found {} chain options! Parsing target options array...",
                    count
                );

                let mut offset = 20;

                for chain_option in 0..count {
                    if offset + 7 <= messages.len() {
                        let card_code = u32::from_le_bytes([
                            messages[offset],
                            messages[offset + 1],
                            messages[offset + 2],
                            messages[offset + 3],
                        ]);

                        let controller = CardController::try_from(messages[offset + 4]).unwrap();
                        let location_bit = messages[offset + 5];
                        let sequence = messages[offset + 6];

                        // Safe decoding pattern to prevent unwrap crashes on unexpected bytes
                        let location = CardLocation::try_from(location_bit).unwrap();

                        debug!(
                            "  Option #{}: Card ID {}, Controller: {:?}, Location: {:?}, Slot: {}",
                            chain_option, card_code, controller, location, sequence
                        );

                        card_prompting_to_activate.with_mut(|v| v.push(ActiveCard { card_code, controller, location, sequence, chain_option: Some(chain_option as u8) }))
                    }

                    // Advance by 23 bytes to cleanly clear the trailing descriptive payload fields
                    offset += 23;
                }

                waiting_on_input.set(true);
            }
        }
        CoreMessage::SelectEffectYN => {
            if messages.len() < 18 {
                debug!("SelectEffectYN packet too small: {} bytes", messages.len());
                return;
            }

            let player = messages[5];

            // Parse the target card triggering its choice prompt
            let card_code =
                u32::from_le_bytes([messages[6], messages[7], messages[8], messages[9]]);
            let location = CardLocation::try_from(messages[11]).unwrap();
            let sequence = messages[12];

            debug!(
                "SelectEffectYN -> Player: {}, Card: {}, Location: {:?}, Zone Index: {}",
                player, card_code, location, sequence
            );

            card_prompting_to_activate.with_mut(|v| v.push(ActiveCard { card_code, controller: CardController::Player, location, sequence, chain_option: None }));
            waiting_on_input.set(true);
        }
        CoreMessage::SelectCard => {}
    }

    monsters.set(duel.get_cards(CardLocation::MonsterZone));
    hand_contents.set(duel.get_cards(CardLocation::Hand));
}
