use dioxus::prelude::*;
use std::collections::HashMap;

use super::card::Card;
use super::svg::SummonIcon;
use crate::ocgcore::HandAction;

#[component]
pub fn Hand(
    cards: ReadSignal<Vec<u32>>,
    selected_card: WriteSignal<i32>,
    normal_summons: ReadSignal<HashMap<u8, u16>>,
    hand_chainables: ReadSignal<HashMap<u8, usize>>,
    hand_actions: Callback<HandAction>,
) -> Element {
    let hand_size = cards().len() as i32;
    let center = hand_size - 1;

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[25%]",
            for (index, card_id) in cards().iter().copied().enumerate() {
                {
                    let summon_index = normal_summons().get(&(index as u8)).copied();
                    let chainable = hand_chainables().get(&(index as u8)).copied().is_some();
                    let distance = (index as i32) * 2 - center;
                    let abs_distance = distance.abs();
                    let mut rotation = distance;
                    let mut translate_y = (abs_distance * abs_distance * 4) / 16;
                    let mut z_index = 0;
                    let is_selected = selected_card() == index as i32;
                    let summonable = summon_index.is_some();

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
                            onclick: move |_| selected_card.set(index as i32),

                            if summon_index.is_some() {
                                div {
                                    class: "absolute -inset-[5px] rounded-[4px] bg-cyan-400 blur-[2px] mix-blend-screen pointer-events-none -z-10"
                                }
                            }
                            if chainable {
                                div {
                                    class: "absolute -inset-[5px] rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none -z-10"
                                }
                            }

                            Card {
                                id: card_id
                            }

                            if summon_index.is_some() {
                                div {
                                    class: "absolute inset-0 border-5 border-cyan-300/50 blur-[2px] mix-blend-screen pointer-events-none z-20 animate-pulse"
                                }
                            }
                            if chainable {
                                div {
                                    class: "absolute inset-0 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none z-20 animate-pulse"
                                }
                            }

                            // Normal Summon button above selected card
                            if is_selected {
                                if summonable || chainable {
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
                                                        hand_actions.call(HandAction::NormalSummon {
                                                            card_code: card_id,
                                                            summon_index: summon_index.unwrap(),
                                                        });
                                                    },
                                                    SummonIcon {}
                                                }
                                            }
                                        }
                                        if chainable {
                                            div {
                                                p {
                                                    class: "text-white font-semibold shadow-md",
                                                    "Activate"
                                                }
                                                button {
                                                    class: "bg-black size-16 p-2 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer",
                                                    onclick: move |evt| {
                                                        evt.stop_propagation();
                                                        hand_actions.call(HandAction::Chain {
                                                            card_code: card_id,
                                                            sequence: hand_chainables().get(&(index as u8)).copied().unwrap() as u8,
                                                        });
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
