use dioxus::prelude::*;

use super::components::ActionButton;
use super::components::Card;
use super::components::CardActionMenu;
use super::components::OptionButton;
use super::components::PickerModal;
use super::components::svg::SummonIcon;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_user_response;
use crate::ui::components::CardStack;

#[component]
pub fn Graveyard() -> Element {
    let mut state = use_context::<DuelState>();

    let has_cards = state.graveyard.len() > 0;
    let has_trigger_effects = state
        .card_prompting_to_activate
        .iter()
        .any(|card| card.location == CardLocation::Graveyard);

    rsx!(
        div {
            class: "relative bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            class: if has_trigger_effects {"outline-4 outline-yellow-300/50"},
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards { state.show_graveyard.set(true) },
            if has_cards {
                CardStack {
                    length: state.graveyard.len(),
                    image_url: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", (state.graveyard)().last().unwrap().unwrap().card_code),
                }
            }
        }
    )
}

#[component]
pub fn GraveyardModal() -> Element {
    let state = use_context::<DuelState>();
    let graveyard = state.graveyard;
    let cards_prompting_to_activate = state.card_prompting_to_activate;
    let mut show_graveyard = state.show_graveyard;

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Graveyard",
            trigger: show_graveyard(),
            div {
                class: "flex flex-row gap-2 min-w-[40vw] w-[40vw] max-w-[40vw]",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in graveyard().iter().enumerate() {
                    {
                        let prompted_card = cards_prompting_to_activate()
                            .iter()
                            .find(|card| card.location == CardLocation::Graveyard && card.index == index as u8)
                            .copied();
                        let chain_option = prompted_card.and_then(|card| card.chain_option);

                        rsx!(
                            div {
                                class: "relative p-2",
                                Card {
                                    code: card.unwrap().card_code,
                                    class: "w-[12vw]",
                                    is_selected: selected_card() == Some(index),
                                    highlight_on_select: true,
                                    is_normal_summonable: false,
                                    is_activatable: prompted_card.is_some(),
                                    onclick: move |_| selected_card.set(Some(index))
                                }
                                CardActionMenu {
                                    class: "absolute left-1/2 -translate-x-[50%] -translate-y-[96px]",
                                    trigger: selected_card() == Some(index) && prompted_card.is_some(),
                                    ActionButton {
                                        label: "Activate",
                                        class: "border-yellow-500 text-yellow-300",
                                        onclick: move |_| {
                                            if prompted_card.is_some() {
                                                if let Some(chain_option) = chain_option {
                                                    send_user_response(UserResponse::Chain { index: chain_option });
                                                } else {
                                                    send_user_response(UserResponse::Yes);
                                                }

                                                // if activatable {
                                                //     send_user_response(UserResponse::Activate { index: activatable_eff_index as u8 });
                                                // }

                                                selected_card.set(None);
                                            }
                                        },
                                        SummonIcon {  }
                                    }
                                }
                            }
                        )
                    }

                }
            }
            OptionButton {
                label: "Close",
                onclick: move |_| {
                    show_graveyard.set(false);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}
