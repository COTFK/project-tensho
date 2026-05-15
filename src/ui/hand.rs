use super::card::Card;
use dioxus::prelude::*;

#[component]
pub fn Hand(
    cards: ReadSignal<Vec<String>>,
    selected_card: WriteSignal<i32>,
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
