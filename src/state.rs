use dioxus::prelude::*;
use std::collections::HashMap;

use crate::ocgcore::ActiveCard;
use crate::ocgcore::CoreMessage;
use crate::ocgcore::Duel;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::utility::get_cached_label;

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedCard {
    pub location: CardLocation,
    pub index: u8,
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
    pub selectables: Signal<Vec<ActiveCard>>,
    pub yes_no_question: Signal<Option<String>>,
}

pub fn send_user_response(response: UserResponse) {
    let mut state = use_context::<DuelState>();

    state.duel.set_response(response);

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
    let mut selectables = state.selectables;
    let mut yes_no_question = state.yes_no_question;

    match duel.parse_messages() {
        CoreMessage::Retry => {
            warn!("Received Retry - this shouldn't happen.");
        }
        CoreMessage::Idle(actions) => {
            normal_summons.set(actions.get_normal_summons());
        }
        CoreMessage::SelectPlace => send_user_response(UserResponse::Place {
            controller: CardController::Player as u8,
            location: CardLocation::MonsterZone as u8,
            index: 0,
        }),
        CoreMessage::SelectChain(effects) => {
            if effects.is_empty() {
                send_user_response(UserResponse::PassPriority);
            } else {
                card_prompting_to_activate.set(effects);
                waiting_on_input.set(true);
            }
        }
        CoreMessage::SelectEffectYN(effect) => {
            card_prompting_to_activate.with_mut(|v| v.push(effect));
            waiting_on_input.set(true);
        }
        CoreMessage::SelectCard(received_selectables) => {
            debug!("selecting from {:?}", received_selectables);
            selectables.set(received_selectables);
            waiting_on_input.set(true);
        }
        CoreMessage::SelectYesNo {
            player: _,
            card_code,
            string_index,
        } => {
            debug!("got {card_code}, {string_index}");

            let label = get_cached_label(card_code).unwrap();
            yes_no_question.set(Some(
                label
                    .optional_strings
                    .get(&string_index)
                    .unwrap()
                    .to_owned(),
            ));
        }
    }

    monsters.set(duel.get_cards(CardLocation::MonsterZone));
    hand_contents.set(duel.get_cards(CardLocation::Hand));
}
