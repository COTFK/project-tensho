use crate::card::Card;
use dioxus::prelude::*;

#[component]
pub fn Hand(
    cards: ReadSignal<Vec<String>>,
    selected_card: WriteSignal<i32>,
    #[props(default)] available_summons: ReadSignal<Vec<u32>>,
    #[props(default)] on_normal_summon: Callback<()>,
) -> Element {
    let hand_size = cards().len() as i32;
    let center = hand_size - 1;

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[25%]",
            for (index, card_id) in cards().iter().enumerate() {
                {
                    let distance = (index as i32) * 2 - center;
                    let abs_distance = distance.abs();
                    let mut rotation = distance;
                    let mut translate_y = (abs_distance * abs_distance * 4) / 16;
                    let mut z_index = 0;
                    let is_selected = selected_card() == index as i32;

                    if is_selected {
                        translate_y = -25;
                        rotation = 0;
                        z_index = 100;
                    }

                    rsx! {
                        div {
                            id: index,
                            class: "transform-gpu transition duration-100 ease-in-out -mx-[2%] relative",
                            transform: "rotateZ({rotation}deg) translateY({translate_y}%)",
                            z_index: z_index,
                            onclick: move |_| selected_card.set(index as i32),

                            // Normal Summon button above selected card
                            if is_selected {
                                {
                                    let card_code: u32 = card_id.parse().unwrap_or(0);
                                    let can_summon = available_summons().contains(&card_code);
                                    
                                    if can_summon {
                                        rsx! {
                                            div {
                                                class: "absolute -top-16 left-1/2 transform -translate-x-1/2 z-50",
                                                button {
                                                    class: "px-4 py-2 bg-gradient-to-b from-yellow-400 to-yellow-600 hover:from-yellow-300 hover:to-yellow-500 text-black font-bold rounded-lg shadow-lg hover:shadow-xl transition-all duration-200 text-sm whitespace-nowrap border-2 border-yellow-700 active:scale-95",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        on_normal_summon.call(());
                                                    },
                                                    "Normal Summon"
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }

                            Card {
                                id: card_id
                            }
                        }
                    }
                }
            }
        }

    )
}
