use dioxus::prelude::*;
use std::collections::HashMap;

use super::field::Field;
use super::hand::Hand;
use super::modal::ModalContainer;
use crate::ocgcore::Duel;
use crate::ocgcore::DuelStatus;
use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::handle_core_message;
use crate::state::send_user_response;

#[component]
pub fn DuelScreen(duel: Duel) -> Element {
    // Load everything into the struct and pass it on
    let state = DuelState {
        duel,
        hand_contents: use_signal(Vec::new),
        selected_card: use_signal(|| None),
        normal_summons: use_signal(HashMap::new),
        activatable_effects: use_signal(HashMap::new),
        waiting_on_input: use_signal(|| false),
        monsters: use_signal(Vec::new),
        spell_traps: use_signal(Vec::new),
        card_prompting_to_activate: use_signal(Vec::new),
        selectables: use_signal(|| Vec::new()),
        yes_no_question: use_signal(|| None),
        available_zones: use_signal(Vec::new),
        positions_to_select: use_signal(Vec::new)
    };
    use_context_provider(|| state);

    // Main game loop
    use_effect(move || {
        let state = use_context::<DuelState>();

        let waiting_on_input = state.waiting_on_input;
        let duel = state.duel;

        if !waiting_on_input() {
            loop {
                match duel.process() {
                    DuelStatus::Awaiting => {
                        handle_core_message();
                        break;
                    }
                    DuelStatus::Continue => continue,
                    DuelStatus::End => break,
                }
            }
        }
    });

    // Add various behaviors to right clicks
    let right_click_handler = move |evt: MouseEvent| {
        let state = use_context::<DuelState>();

        // Allow to decline chains & activations
        let mut card_prompting_to_activate = state.card_prompting_to_activate;
        let is_chainable = card_prompting_to_activate
            .iter()
            .any(|card| card.chain_option.is_some());

        if !card_prompting_to_activate().is_empty() {
            if is_chainable {
                send_user_response(UserResponse::PassPriority)
            } else {
                send_user_response(UserResponse::No);
            }

            card_prompting_to_activate.with_mut(|v| v.clear());
        }

        // Decline Yes/No questions
        let mut yes_no_question = state.yes_no_question;
        if yes_no_question().is_some() {
            send_user_response(UserResponse::No);
            yes_no_question.set(None);
        }

        evt.prevent_default();
    };

    let left_click_handler = move |_evt: MouseEvent| {
        let mut state = use_context::<DuelState>();
        if (state.selected_card)().is_some() {
            state.selected_card.set(None);
        }
    };

    rsx!(
        main {
            class: "h-dvh w-dvw bg-gray-800",
            oncontextmenu: right_click_handler,
            onclick: left_click_handler,
            ModalContainer {}
            Field {}
            Hand {}
        }
    )
}
