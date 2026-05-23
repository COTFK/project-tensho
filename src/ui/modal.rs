use dioxus::prelude::*;

use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_user_response;
use super::svg::SummonIcon;

#[component]
pub fn ModalContainer() -> Element {
    let state = use_context::<DuelState>();
    let mut picker_selected_card = use_signal(|| None);

    rsx!(
        Modal {
            enabled: state.card_prompting_to_activate.iter().any(|card| card.chain_option.is_some()),
            message: "A card or effect can be activated. Activate?",
            button {
                class: "w-32 h-8 bg-red-600/70 rounded-lg font-semibold text-white",
                onclick: |_| send_user_response(UserResponse::No),
                "No"
            }
        }
        Modal {
            enabled: !state.card_prompting_to_activate.iter().any(|card| card.chain_option.is_some()) && !state.card_prompting_to_activate.is_empty(),
            message: "Activate trigger effect?",
            button {
                class: "w-32 h-8 bg-red-600/70 rounded-lg font-semibold text-gray-300",
                onclick: |_| send_user_response(UserResponse::No),
                "No"
            }
        }
        Modal {
            enabled: (state.yes_no_question)().is_some(),
            message: (state.yes_no_question)().unwrap_or(String::new()),
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
            enabled: !state.selectables.is_empty(),
            message: "Select a card",
            vertical: true,
            div {
                class: "flex flex-row gap-2 overflow-x-auto overflow-y-hidden w-full flex-1 min-h-0 items-stretch",
                for card in (state.selectables)() {
                    div {
                        class: "border-2 h-full flex-none",
                        class: if picker_selected_card() == Some(card.sequence) { "border-yellow-300" } else { "border-transparent" },
                        onclick: move |_| {
                            picker_selected_card.set(Some(card.sequence));
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
                    class: "h-10 w-24 rounded-lg font-semibold text-white mx-auto",
                    class: if !picker_selected_card().is_some() { "bg-gray-600 cursor-not-allowed" } else { "bg-green-700 cursor-pointer" },
                    disabled: !picker_selected_card().is_some(),
                    onclick: move |_| send_user_response(UserResponse::SelectCard { sequence: picker_selected_card.unwrap() }),
                    "Done"
                }
            }
        }
        Modal {
            enabled: !state.positions_to_select.is_empty(),
            message: "Select battle position",
            vertical: true,
            {
                rsx!(
                    for position in (state.positions_to_select)() {
                        button {
                            class: "h-12 w-full rounded-lg font-semibold text-white mx-auto bg-gray-600 cursor-pointer",
                            onclick: move |_| send_user_response(UserResponse::SelectPosition { position }),
                            {position.to_string()}
                        }
                    }
                )
            }
        }
        GraveyardModal { }
    )
}

#[component]
pub fn Modal(enabled: bool, message: String, children: Element, vertical: Option<bool>) -> Element {
    rsx!(
        div {
            class: "absolute left-1/2 -translate-x-[50%] z-40",
            class: "max-h-[70vh] w-full p-4 rounded-lg",
            class: if Some(true) == vertical { "flex flex-col items-stretch justify-start gap-4 h-full max-w-[70%]" } else { "flex items-center justify-between max-w-[90%]" },
            class: "bg-gray-700/80 transition-all duration-300 ease-in-out",
            class: if enabled { "top-[2vh] shadow-xl " } else { "-top-[100%] shadow-none" },
            p {
                class: "text-white font-semibold text-gray-300",
                "{message}"
            }
            {children}
        }
    )
}

#[component]
fn GraveyardModal() -> Element {
    let mut state = use_context::<DuelState>();
    let mut gy_selected_card: Signal<Option<usize>> = use_signal(|| None);

    rsx!(
        Modal {
            enabled: (state.show_graveyard)(),
            message: "Graveyard",
            vertical: true,
            div {
                class: "flex flex-row gap-2 overflow-x-auto overflow-y-hidden w-full flex-1 max-h-[40vh] items-stretch p-4",
                for (index, card) in state.graveyard.iter().enumerate() {
                    {
                        let prompted_card = state
                            .card_prompting_to_activate
                            .iter()
                            .find(|card| card.location == CardLocation::Graveyard && card.sequence == index as u8);
                        let prompted = prompted_card.is_some();
                        let chain_option = prompted_card.and_then(|card| card.chain_option);

                            rsx!(
                            div {
                                class: "relative border-2 h-full flex-none",
                                   class: if gy_selected_card() == Some(index) { "border-yellow-300" } else { "border-transparent" },
                                onclick: move |_| {
                                    gy_selected_card.set(Some(index));
                                },
                                if prompted {
                                    div {
                                        class: "absolute -inset-[4px] rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none"
                                    }
                                }
                                   img {
                                    class: "h-full w-auto max-h-none relative",
                                    image_rendering: "smooth",
                                    aspect_ratio: "59/86",
                                    src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card),
                                }
                                if prompted && gy_selected_card() == Some(index) {
                                    div {
                                        class: "absolute inset-0 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none animate-pulse"
                                    }
                                    div {
                                        class: "relative bottom-[50%] flex flex-col items-center justify-center",
                                        div {
                                            p {
                                                class: "text-white font-semibold shadow-md",
                                                "Activate"
                                            }
                                            button {
                                                class: "bg-black size-16 p-2 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer relative",
                                                onclick: move |evt| {
                                                    evt.stop_propagation();

                                                    if prompted {
                                                        if let Some(chain_option) = chain_option {
                                                            send_user_response(UserResponse::Chain { sequence: chain_option });
                                                        } else {
                                                            send_user_response(UserResponse::Yes);
                                                        }
                                                    }

                                                    // if activatable {
                                                    //     send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
                                                    // }
                                                },
                                                SummonIcon {}
                                            }
                                        }
                                    }
                                }
                            }
                        )
                    }
                }
            }
            div {
                class: "flex flex-row",
                button {
                    class: "h-10 w-24 rounded-lg font-semibold text-white mx-auto bg-green-700 cursor-pointer",
                    onclick: move |_| {
                        gy_selected_card.set(None);
                        state.show_graveyard.set(false);
                    },
                    "Close"
                }
            }
        }
    )
}
