use dioxus::prelude::*;

use super::components::ActionButton;
use super::components::OptionButton;
use super::components::Card;
use super::components::CardActionMenu;
use super::components::PickerModal;
use super::components::svg::SummonIcon;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_user_response;

#[component]
pub fn Graveyard() -> Element {
    let mut state = use_context::<DuelState>();

    let any_trigger_effects_in_gy = state
        .card_prompting_to_activate
        .iter()
        .any(|card| card.location == CardLocation::Graveyard);

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5 hover:outline-4 hover:outline-yellow-300",
            class: if any_trigger_effects_in_gy {"outline-4 outline-yellow-300/50"},
            onclick: move |_| state.show_graveyard.set(true),
            for (index, card) in (state.graveyard)().iter().enumerate() {
                div {
                    class: "absolute inset-[clamp(2px,0.6vw,8px)]",
                    img {
                        class: "w-full h-full object-contain",
                        style: "transform: translate({index as f32 * 0.01}vw, -{index as f32 * 0.01}vh);",
                        image_rendering: "smooth",
                        aspect_ratio: "59/86",
                        src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.unwrap().card_code),
                    }
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
                            .find(|card| card.location == CardLocation::Graveyard && card.sequence == index as u8)
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
                                                    send_user_response(UserResponse::Chain { sequence: chain_option });
                                                } else {
                                                    send_user_response(UserResponse::Yes);
                                                }

                                                // if activatable {
                                                //     send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
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
