use dioxus::prelude::*;

use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::send_user_response;
use super::card::Card;

#[component]
pub fn CardPicker() -> Element {
    let state = use_context::<DuelState>();
    let mut selectables = state.selectables;
    let mut selected_card = use_signal(|| None);

    rsx!(
        div {
            class: "z-100 absolute max-w-1/2 max-h-3/4 h-fit bg-gray-700 rounded-xl transition-all flex flex-col duration-300 ease-in-out justify-between mx-auto",
            class: if !selectables.is_empty() { "shadow-xl inset-0" } else { "shadow-none -top-[100%] inset-x-0" },
            p {
                class: "text-white font-semibold p-4 bg-gray-800/70",
                "Select a card"
            }
            div {
                class: "flex flex-row gap-2 m-4 overflow-x-auto",
                for card in selectables() {
                    div {
                        class: "flex-grow border-2",
                        class: if selected_card() == Some(card.sequence) { "border-yellow-300" } else { "border-transparent" },
                        onclick: move |_| {
                            selected_card.set(Some(card.sequence));
                        },
                        Card {
                            id: card.card_code
                        }
                    }
                }
            }
            div {
                class: "flex flex-row p-4 bg-gray-800/70",
                button {
                    class: "h-12 w-1/2 rounded-lg font-semibold text-white mx-auto",
                    class: if !selected_card().is_some() { "bg-gray-600 cursor-not-allowed" } else { "bg-green-700 cursor-pointer" },
                    disabled: !selected_card().is_some(),
                    onclick: move |_| {
                        send_user_response(UserResponse::SelectCard { sequence: selected_card.unwrap() });
                        selectables.clear();
                    },
                    "Done"
                }
            }
        }
    )
}