use dioxus::prelude::*;

use super::svg::SummonIcon;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::SelectedCard;
use crate::state::send_user_response;

#[component]
pub fn Hand() -> Element {
    let state = use_context::<DuelState>();
    let cards = state.hand_contents;
    let normal_summons = state.normal_summons;
    let activatable_effects = (state.activatable_effects)();
    let chainables = state.card_prompting_to_activate;
    let mut selected_card = state.selected_card;

    let hand_size = cards().len() as i32;
    let center = hand_size - 1;

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[25%]",
            for (index, card_id) in cards().iter().copied().enumerate() {
                {
                    let summon_index = normal_summons().get(&(index as u8)).copied();
                    let activatable_card = activatable_effects.iter().find(|(_activate_index, card)| card.location == CardLocation::Hand && card.sequence == index as u8);
                    let activatable_index = if activatable_card.is_some() {
                        let inner = activatable_card.unwrap();
                        *inner.0
                    } else {
                        0
                    };

                    let prompted_card = chainables
                        .iter()
                        .find(|card| card.location == CardLocation::Hand && card.sequence == index as u8);
                    let chainable_id_opt = prompted_card.and_then(|card| card.chain_option);
                    let chainable = chainable_id_opt.is_some();
                    let distance = (index as i32) * 2 - center;
                    let abs_distance = distance.abs();
                    let mut rotation = distance;
                    let mut translate_y = (abs_distance * abs_distance * 4) / 16;
                    let mut z_index = 0;

                    let is_selected = match selected_card() {
                        Some(card) => card.index == index as u8 && card.location == CardLocation::Hand,
                        None => false
                    };

                    let summonable = summon_index.is_some();
                    let activatable = activatable_card.is_some();

                    let chainable_id = chainable_id_opt.unwrap_or(0);

                    if is_selected {
                        translate_y = -25;
                        rotation = 0;
                        z_index = 100;
                    }

                    rsx! {
                        div {
                            key: "{card_id}-{index}",
                            id: index,
                            width: "12.5vw",
                            class: "transform-gpu transition duration-150 ease-in-out -mx-[1.1vw] relative group",
                            transform: "rotateZ({rotation}deg) translateY({translate_y}%)",
                            z_index: z_index,
                            onclick: move |evt| {
                                evt.stop_propagation();
                                selected_card.set(Some(SelectedCard{
                                    location: CardLocation::Hand,
                                    index: index as u8
                                }));
                            },

                            if summon_index.is_some() {
                                div {
                                    class: "absolute -inset-[5px] rounded-[4px] bg-cyan-400 blur-[2px] mix-blend-screen pointer-events-none -z-10"
                                }
                            }
                            if chainable || activatable {
                                div {
                                    class: "absolute -inset-[5px] rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none -z-10"
                                }
                            }

                            img {
                                image_rendering: "smooth",
                                aspect_ratio: "59/86",
                                src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card_id),
                            }

                            if summon_index.is_some() {
                                div {
                                    class: "absolute inset-0 border-5 border-cyan-300/50 blur-[2px] mix-blend-screen pointer-events-none z-20 animate-pulse"
                                }
                            }
                            if chainable || activatable {
                                div {
                                    class: "absolute inset-0 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none z-20 animate-pulse"
                                }
                            }

                            // Normal Summon button above selected card
                            if is_selected {
                                if summonable || chainable || activatable {
                                    div {
                                        class: "absolute -top-26 left-1/2 transform -translate-x-1/2 z-50 flex flex-col items-center justify-center",
                                        if summonable {
                                            div {
                                                p {
                                                    class: "text-white font-semibold shadow-md",
                                                    "Summon"
                                                }
                                                button {
                                                    class: "bg-black size-16 p-2 rounded-full border-3 border-cyan-500 text-cyan-300 cursor-pointer",
                                                    onclick: move |evt| {
                                                        evt.stop_propagation();
                                                        send_user_response(UserResponse::NormalSummon { sequence: summon_index.unwrap() as u8 });
                                                    },
                                                    SummonIcon {}
                                                }
                                            }
                                        }
                                        if chainable || activatable {
                                            div {
                                                p {
                                                    class: "text-white font-semibold shadow-md",
                                                    "Activate"
                                                }
                                                button {
                                                    class: "bg-black size-16 p-2 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer",
                                                    onclick: move |evt| {
                                                        evt.stop_propagation();
                                                        if chainable {
                                                            send_user_response(UserResponse::Chain { sequence: chainable_id });
                                                        }
                                                        if activatable {
                                                            send_user_response(UserResponse::Activate { sequence: activatable_index as u8 });
                                                        }
                                                        
                                                    },
                                                    SummonIcon {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    )
}
