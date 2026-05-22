use dioxus::prelude::*;

use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::send_user_response;

#[component]
pub fn ModalContainer() -> Element {
    let state = use_context::<DuelState>();
    let card_ids = state.card_prompting_to_activate;
    let selectables = state.selectables;
    let yes_no_question = state.yes_no_question;
    let positions_to_select = state.positions_to_select;

    let mut selected_card = use_signal(|| None);

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
        Modal { // Card picker
            enabled: !selectables.is_empty(),
            message: "Select a card",
            vertical: true,
            div {
                class: "flex flex-row gap-2 overflow-x-auto overflow-y-hidden w-full flex-1 min-h-0 items-stretch",
                for card in selectables() {
                    div {
                        class: "border-2 h-full flex-none",
                        class: if selected_card() == Some(card.sequence) { "border-yellow-300" } else { "border-transparent" },
                        onclick: move |_| {
                            selected_card.set(Some(card.sequence));
                        },
                        img {
                            class: "h-full w-auto max-h-none",
                            image_rendering: "smooth",
                            aspect_ratio: "59/86",
                            src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.card_code),
                        }
                    }
                }
            }
            div {
                class: "flex flex-row",
                button {
                    class: "h-12 w-32 rounded-lg font-semibold text-white mx-auto",
                    class: if !selected_card().is_some() { "bg-gray-600 cursor-not-allowed" } else { "bg-green-700 cursor-pointer" },
                    disabled: !selected_card().is_some(),
                    onclick: move |_| send_user_response(UserResponse::SelectCard { sequence: selected_card.unwrap() }),
                    "Done"
                }
            }
        }
        Modal {
            enabled: !positions_to_select.is_empty(),
            message: "Select battle position",
            vertical: true,
            {
                rsx!(
                    for position in positions_to_select() {
                        button {
                            class: "h-12 w-full rounded-lg font-semibold text-white mx-auto bg-gray-600 cursor-pointer",
                            onclick: move |_| send_user_response(UserResponse::SelectPosition { position }),
                            {position.to_string()}
                        }
                    }
                )
            }
        }
    )
}

#[component]
pub fn Modal(enabled: bool, message: String, children: Element, vertical: Option<bool>) -> Element {
    rsx!(
        div {
            class: "absolute left-1/2 -translate-x-[50%] z-100",
            class: "max-h-[70vh] max-w-[60%] w-full p-4 rounded-lg",
            class: if Some(true) == vertical { "flex flex-col items-stretch justify-start gap-4 h-full" } else { "flex items-center justify-between" },
            class: "bg-gray-700/80 transition-all duration-300 ease-in-out",
            class: if enabled { "top-[2vh] shadow-xl " } else { "-top-[100%] shadow-none" },
            p {
                class: "text-white font-semibold",
                "{message}"
            }
            {children}
        }
    )
}
