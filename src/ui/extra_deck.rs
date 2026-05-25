use dioxus::prelude::*;

use super::components::CardStack;
use crate::state::DuelState;
use crate::utility::EXTRA_BACK;
use super::components::PickerModal;
use super::components::ActionButton;
use super::components::Card;
use super::components::CardActionMenu;
use super::components::OptionButton;

#[component]
pub fn ExtraDeck() -> Element {
    let mut state = use_context::<DuelState>();

    let has_cards = state.extra_deck.len() > 0;

    rsx!(
        div {
            class: "relative bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards { state.show_extra_deck.set(true) },
            CardStack {
                length: state.extra_deck.len(),
                image_url: EXTRA_BACK,
            }
        }
    )
}

#[component]
pub fn ExtraDeckModal() -> Element {
    let state = use_context::<DuelState>();
    let extra_deck = state.extra_deck;
    let mut show_extra_deck = state.show_extra_deck;

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Extra Deck",
            trigger: show_extra_deck(),
            div {
                class: "flex flex-row min-w-[40vw] gap-[0.5vw] w-full max-w-[76vw]",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in extra_deck().iter().enumerate() {
                    {
                        rsx!(
                            div {
                                class: "relative",
                                Card {
                                    code: card.unwrap().card_code,
                                    class: "w-[8vw]",
                                    is_selected: selected_card() == Some(index),
                                    highlight_on_select: true,
                                    is_normal_summonable: false,
                                    is_activatable: false,
                                    onclick: move |_| selected_card.set(Some(index))
                                }
                                // CardActionMenu {
                                //     class: "absolute left-1/2 -translate-x-[50%] -translate-y-[96px]",
                                //     trigger: selected_card() == Some(index) && prompted_card.is_some(),
                                //     ActionButton {
                                //         label: "Activate",
                                //         class: "border-yellow-500 text-yellow-300",
                                //         onclick: move |_| {
                                //             if prompted_card.is_some() {
                                //                 if let Some(chain_option) = chain_option {
                                //                     send_user_response(UserResponse::Chain { sequence: chain_option });
                                //                 } else {
                                //                     send_user_response(UserResponse::Yes);
                                //                 }

                                //                 // if activatable {
                                //                 //     send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
                                //                 // }

                                //                 selected_card.set(None);
                                //             }
                                //         },
                                //         SummonIcon {  }
                                //     }
                                // }
                            }
                        )
                    }

                }
            }
            OptionButton {
                label: "Close",
                onclick: move |_| {
                    show_extra_deck.set(false);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}

// #[component]
// pub fn extra_deckModal() -> Element {
//     let state = use_context::<DuelState>();
//     let extra_deck = state.extra_deck;
//     let cards_prompting_to_activate = state.card_prompting_to_activate;
//     let mut show_extra_deck = state.show_extra_deck;

//     let mut selected_card = use_signal(|| None);

//     rsx!(
//         PickerModal {
//             title: "extra_deck",
//             trigger: show_extra_deck(),
//             div {
//                 class: "flex flex-row gap-2 min-w-[40vw] w-[40vw] max-w-[40vw]",
//                 class: "overflow-x-auto scroll-smooth scrollbar-thin",
//                 for (index, card) in extra_deck().iter().enumerate() {
//                     {
//                         let prompted_card = cards_prompting_to_activate()
//                             .iter()
//                             .find(|card| card.location == CardLocation::extra_deck && card.sequence == index as u8)
//                             .copied();
//                         let chain_option = prompted_card.and_then(|card| card.chain_option);

//                         rsx!(
//                             div {
//                                 class: "relative p-2",
//                                 Card {
//                                     code: card.unwrap().card_code,
//                                     class: "w-[12vw]",
//                                     is_selected: selected_card() == Some(index),
//                                     highlight_on_select: true,
//                                     is_normal_summonable: false,
//                                     is_activatable: prompted_card.is_some(),
//                                     onclick: move |_| selected_card.set(Some(index))
//                                 }
//                                 CardActionMenu {
//                                     class: "absolute left-1/2 -translate-x-[50%] -translate-y-[96px]",
//                                     trigger: selected_card() == Some(index) && prompted_card.is_some(),
//                                     ActionButton {
//                                         label: "Activate",
//                                         class: "border-yellow-500 text-yellow-300",
//                                         onclick: move |_| {
//                                             if prompted_card.is_some() {
//                                                 if let Some(chain_option) = chain_option {
//                                                     send_user_response(UserResponse::Chain { sequence: chain_option });
//                                                 } else {
//                                                     send_user_response(UserResponse::Yes);
//                                                 }

//                                                 // if activatable {
//                                                 //     send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
//                                                 // }

//                                                 selected_card.set(None);
//                                             }
//                                         },
//                                         SummonIcon {  }
//                                     }
//                                 }
//                             }
//                         )
//                     }

//                 }
//             }
//             OptionButton {
//                 label: "Close",
//                 onclick: move |_| {
//                     show_extra_deck.set(false);
//                     selected_card.set(None);
//                 },
//                 additional_classes: "bg-green-600/70"
//             }
//         }
//     )
// }
