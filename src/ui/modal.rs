use dioxus::prelude::*;

use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::send_user_response;

#[component]
pub fn ChainQuestionModal() -> Element {
    let state = use_context::<DuelState>();
    let mut card_ids = state.card_prompting_to_activate;
    let enabled = card_ids.iter().any(|card| card.chain_option.is_some());

    rsx!(
        div {
            class: "absolute inset-0 w-[50vw] h-[8vh] bg-gray-500/25 mx-auto rounded-lg transition-all flex flex-row items-center px-4 duration-300 ease-in-out justify-between",
            class: if enabled { "top-[2vh] shadow-xl" } else { "-top-[8vh] shadow-none" },
            p {
                class: "text-white font-semibold",
                "A card or effect can be activated. Activate?"
            }
            button {
                class: "h-2/3 w-24 bg-red-600 rounded-lg font-semibold",
                onclick: move |_| {
                    send_user_response(UserResponse::No);
                    card_ids.clear();
                },
                "Cancel"
            }
        }
    )
}

#[component]
pub fn TriggerModal() -> Element {
    let state = use_context::<DuelState>();
    let mut card_ids = state.card_prompting_to_activate;

    // Show only for single trigger effects
    let enabled = !card_ids.iter().any(|card| card.chain_option.is_some()) && !card_ids.is_empty();

    rsx!(
        div {
            class: "absolute inset-0 w-[50vw] h-[8vh] bg-gray-500/25 mx-auto rounded-lg transition-all flex flex-row items-center px-4 duration-300 ease-in-out justify-between",
            class: if enabled { "top-[2vh] shadow-xl" } else { "-top-[8vh] shadow-none" },
            p {
                class: "text-white font-semibold",
                "Activate trigger effect?"
            }
            button {
                class: "h-2/3 w-24 bg-red-600 rounded-lg font-semibold",
                onclick: move |_| {
                    send_user_response(UserResponse::No);
                    card_ids.clear();
                },
                "Cancel"
            }
        }
    )
}

#[component]
pub fn YesNoModal() -> Element {
    let state = use_context::<DuelState>();
    let mut yes_no_question = state.yes_no_question;

    rsx!(
        div {
            class: "z-100 absolute max-w-1/2 max-h-3/4 h-fit bg-gray-700/70 rounded-xl transition-all flex flex-col duration-300 ease-in-out justify-between mx-auto",
            class: if yes_no_question().is_some() { "shadow-xl inset-0 top-4" } else { "shadow-none -top-[100%] inset-x-0" },
            p {
                class: "text-white font-semibold p-4",
                "{yes_no_question().unwrap_or(String::new())}"
            }
            div {
                class: "flex flex-row p-4 gap-4",
                button {
                    class: "h-12 w-1/2 rounded-lg font-semibold text-white mx-auto bg-green-700 cursor-pointer",
                    onclick: move |_| {
                        send_user_response(UserResponse::Yes);
                        yes_no_question.set(None);
                    },
                    "Yes"
                }
                button {
                    class: "h-12 w-1/2 rounded-lg font-semibold text-white mx-auto bg-red-700 cursor-pointer",
                    onclick: move |_| {
                        send_user_response(UserResponse::No);
                        yes_no_question.set(None);
                    },
                    "No"
                }
            }
        }
    )
}
