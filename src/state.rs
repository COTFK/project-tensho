use dioxus::prelude::*;
use std::collections::HashMap;

use crate::ocgcore::ActiveCard;
use crate::ocgcore::CoreMessage;
use crate::ocgcore::Duel;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardOwner;
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
    pub main_deck_length: Signal<u32>,
    pub extra_deck_length: Signal<u32>,
    pub hand_contents: Signal<Vec<Option<ActiveCard>>>,
    pub selected_card: Signal<Option<SelectedCard>>,
    pub normal_summons: Signal<HashMap<u8, u16>>,
    pub activatable_effects: Signal<HashMap<u16, ActiveCard>>,
    pub waiting_on_input: Signal<bool>,
    pub monsters: Signal<Vec<Option<ActiveCard>>>,
    pub spell_traps: Signal<Vec<Option<ActiveCard>>>,
    pub graveyard: Signal<Vec<Option<ActiveCard>>>,
    pub card_prompting_to_activate: Signal<Vec<ActiveCard>>,
    pub selectables: Signal<Vec<ActiveCard>>,
    pub yes_no_question: Signal<Option<String>>,
    pub available_zones: Signal<Vec<(CardLocation, u8)>>,
    pub positions_to_select: Signal<Vec<BattlePosition>>,
    pub show_graveyard: Signal<bool>,
}

pub fn send_user_response(response: UserResponse) {
    let mut state = use_context::<DuelState>();

    state.duel.set_response(response);

    // Clean up state and wait for new data
    state.hand_contents.clear();
    state.selected_card.set(None);
    state.normal_summons.clear();
    state.activatable_effects.clear();
    state.waiting_on_input.set(false);
    state.monsters.clear();
    state.spell_traps.clear();
    state.card_prompting_to_activate.clear();
    state.selectables.clear();
    state.yes_no_question.set(None);
    state.available_zones.clear();
    state.positions_to_select.clear();
    state.show_graveyard.set(false);
}

pub fn handle_core_message() {
    let mut state = use_context::<DuelState>();

    match state.duel.parse_messages() {
        CoreMessage::Retry => {
            warn!("Received Retry - this shouldn't happen.");
        }
        CoreMessage::Idle(actions) => {
            state.normal_summons.set(actions.get_normal_summons());
            state
                .activatable_effects
                .set(actions.get_activatable_effects());
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectPlace(zones) => {
            state.available_zones.set(zones);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectChain(effects) => {
            if effects.is_empty() {
                send_user_response(UserResponse::PassPriority);
            } else {
                state.card_prompting_to_activate.set(effects);
                state.waiting_on_input.set(true);
            }
        }
        CoreMessage::SelectEffectYN(effect) => {
            state
                .card_prompting_to_activate
                .with_mut(|v| v.push(effect));
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectCard(received_selectables) => {
            debug!("selecting from {:?}", received_selectables);
            state.selectables.set(received_selectables);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectYesNo {
            player: _,
            card_code,
            string_index,
        } => {
            debug!("got {card_code}, {string_index}");

            let label = get_cached_label(card_code).unwrap();
            state.yes_no_question.set(Some(
                label
                    .optional_strings
                    .get(&string_index)
                    .unwrap_or(&String::from("undefined label"))
                    .to_owned(),
            ));
        }
        CoreMessage::SelectPosition(positions) => {
            state.positions_to_select.set(positions);
            state.waiting_on_input.set(true);
        }
    }

    state.main_deck_length.set(state.duel.count_location(CardOwner::Player, CardLocation::Deck));
    state.extra_deck_length.set(state.duel.count_location(CardOwner::Player, CardLocation::ExtraDeck));

    state
        .monsters
        .set(state.duel.get_cards(CardLocation::MonsterZone));
    state
        .spell_traps
        .set(state.duel.get_cards(CardLocation::SpellTrapZone));
    state
        .hand_contents
        .set(state.duel.get_cards(CardLocation::Hand));
    state
        .graveyard
        .set(state.duel.get_cards(CardLocation::Graveyard));
}
