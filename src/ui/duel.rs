use dioxus::prelude::*;
use std::collections::HashMap;
use web_sys::window;

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
pub fn DuelScreen(duel: Duel, resource_handle: Resource<anyhow::Result<Duel>>) -> Element {
    // Load everything into the struct and pass it on
    let state = DuelState {
        duel,
        main_deck_length: use_signal(|| 0),
        extra_deck_length: use_signal(|| 0),
        hand_contents: use_signal(Vec::new),
        selected_card: use_signal(|| None),
        normal_summons: use_signal(HashMap::new),
        activatable_effects: use_signal(HashMap::new),
        waiting_on_input: use_signal(|| false),
        monsters: use_signal(Vec::new),
        spell_traps: use_signal(Vec::new),
        graveyard: use_signal(Vec::new),
        card_prompting_to_activate: use_signal(Vec::new),
        selectables: use_signal(|| Vec::new()),
        yes_no_question: use_signal(|| None),
        available_zones: use_signal(Vec::new),
        positions_to_select: use_signal(Vec::new),
        show_graveyard: use_signal(|| false),
        effects_to_select_from: use_signal(Vec::new),
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
        let mut state = use_context::<DuelState>();

        // Allow to decline chains & activations
        if !(state.card_prompting_to_activate)().is_empty() {
            if (state.card_prompting_to_activate)()
                .iter()
                .any(|card| card.chain_option.is_some())
            {
                send_user_response(UserResponse::PassPriority)
            } else {
                send_user_response(UserResponse::No);
            }

            state.card_prompting_to_activate.with_mut(|v| v.clear());
        }

        // Decline Yes/No questions
        if (state.yes_no_question)().is_some() {
            send_user_response(UserResponse::No);
            state.yes_no_question.set(None);
        }

        if (state.show_graveyard)() == true {
            state.show_graveyard.set(false);
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
            class: "relative h-dvh w-dvw bg-gray-800",
            oncontextmenu: right_click_handler,
            onclick: left_click_handler,
            ResetButton {
                onclick: move |_| {
                    resource_handle.clear();
                    resource_handle.restart();
                }
            }
            FullscreenButton {}
            ModalContainer {}
            Field {}
            Hand {}
        }
    )
}

#[component]
pub fn FullscreenButton() -> Element {
    let toggle_fullscreen = move |_| {
        if let Some(document) = window().and_then(|window| window.document()) {
            if document.fullscreen_element().is_some() {
                let _ = document.exit_fullscreen();
            } else if let Some(element) = document.document_element() {
                let _ = element.request_fullscreen();
            }
        }
    };

    rsx!(
        button {
            class: "fixed top-3 right-3 z-50 w-8 h-8 rounded-md border border-white/20 bg-black/70 text-[10px] font-semibold text-white shadow-lg backdrop-blur-sm flex items-center justify-center cursor-pointer hover:bg-black/85",
            aria_label: "Toggle fullscreen",
            onclick: toggle_fullscreen,
            svg {
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                class: "w-5 h-5",
                fill: "currentColor",
                stroke: "currentColor",
                stroke_width: "0.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M3 3h6v2H5v4H3V3zm18 0v6h-2V5h-4V3h6zM3 21v-6h2v4h4v2H3zm18-6v6h-6v-2h4v-4h2z" }
            }
        }
    )
}

#[component]
pub fn ResetButton(onclick: EventHandler<MouseEvent>) -> Element {
    rsx!(
        button {
            class: "fixed top-3 left-3 z-50 w-8 h-8 rounded-md border border-white/20 bg-black/70 text-[10px] font-semibold text-white shadow-lg backdrop-blur-sm flex items-center justify-center cursor-pointer hover:bg-black/85",
            aria_label: "Restart",
            onclick: onclick,
            svg {
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                class: "w-5 h-5",
                fill: "none",
                stroke: "currentColor",
                color: "currentColor",
                stroke_width: "2",
                stroke_linecap: "square",
                stroke_linejoin: "miter",
                polyline { points: "22 12 19 15 16 12" }
                path { d: "M11,20 C6.581722,20 3,16.418278 3,12 C3,7.581722 6.581722,4 11,4 C15.418278,4 19,7.581722 19,12 L19,14" }
            }
        }
    )
}
