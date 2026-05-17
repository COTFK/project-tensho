use super::card::Card;
use dioxus::prelude::*;
use std::collections::HashSet;

#[component]
pub fn Hand(
    cards: ReadSignal<Vec<u32>>,
    selected_card: WriteSignal<i32>,
    normal_summons: ReadSignal<HashSet<u32>>,
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
                            class: "transform-gpu transition duration-150 ease-in-out -mx-[1.1vw] relative group",
                            transform: "rotateZ({rotation}deg) translateY({translate_y}%)",
                            z_index: z_index,
                            onclick: move |_| selected_card.set(index as i32),
                            
                            if normal_summons().contains(card_id) {
                                div { 
                                    class: "absolute -inset-[4px] rounded-[4px] bg-cyan-400/90 blur-sm mix-blend-screen pointer-events-none -z-10" 
                                }
                                div { 
                                    class: "absolute -inset-[8px] rounded-xl bg-blue-500/90 blur-md mix-blend-screen pointer-events-none -z-10" 
                                }
                            }

                            Card {
                                id: card_id
                            }

                            if normal_summons().contains(card_id) {
                                div {
                                    // This layer sits strictly *inside* the card dimensions (inset-0)
                                    // We use a high opacity border and a tight inset shadow 
                                    // to make it look like light is bleeding over the frame.
                                    class: "absolute inset-0 border-2 border-cyan-600/70 shadow-[inset_0_0_8px_3px_rgba(34,211,238,0.5)] mix-blend-screen pointer-events-none z-20"
                                }
                            }
                        }
                    }
                }
            }
        }

    )
}
