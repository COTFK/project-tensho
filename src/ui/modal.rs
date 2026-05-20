use dioxus::prelude::*;

use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::send_user_response;
use super::picker::CardPicker;

#[component]
pub fn ModalContainer() -> Element {
    let state = use_context::<DuelState>();
    let card_ids = state.card_prompting_to_activate;
    let yes_no_question = state.yes_no_question;

    rsx!(
        Modal { 
            enabled: card_ids.iter().any(|card| card.chain_option.is_some()),
            message: "A card or effect can be activated. Activate?",
            button {
                class: "w-32 h-8 bg-red-600 rounded-lg font-semibold",
                onclick: |_| send_user_response(UserResponse::No),
                "Cancel"
            }
        }
        Modal { 
            enabled: !card_ids.iter().any(|card| card.chain_option.is_some()) && !card_ids.is_empty(),
            message: "Activate trigger effect?",
            button {
                class: "w-32 h-8 bg-red-600 rounded-lg font-semibold",
                onclick: |_| send_user_response(UserResponse::No),
                "Cancel"
            }
        }
        Modal { 
            enabled: yes_no_question().is_some(),
            message: yes_no_question().unwrap_or(String::new()),
            div {
                class: "flex flex-row gap-4",
                button {
                    class: "w-32 h-10 rounded-lg font-semibold text-white mx-auto bg-green-700 cursor-pointer",
                    onclick: move |_| send_user_response(UserResponse::Yes),
                    "Yes"
                }
                button {
                    class: "w-32 h-10 rounded-lg font-semibold text-white mx-auto bg-red-700 cursor-pointer",
                    onclick: move |_| send_user_response(UserResponse::No),
                    "No"
                }
            }
        }
        CardPicker {}
    )
}

#[component]
pub fn Modal(enabled: bool, message: String, children: Element) -> Element {
    rsx!(
        div {
            class: "absolute left-1/2 -translate-x-[50%]",
            class: "max-w-[60%] w-full p-6 rounded-lg",
            class: "flex flex-row items-center justify-between",
            class: "bg-gray-500/25 transition-all duration-300 ease-in-out",
            class: if enabled { "top-[2vh] shadow-xl " } else { "-top-[100%] shadow-none" },
            p {
                class: "text-white font-semibold",
                "{message}"
            }
            {children}
        }
    )
}
