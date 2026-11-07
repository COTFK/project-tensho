use dioxus::prelude::*;

use super::card_interaction::InteractiveCard;
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::state::UIState;
use crate::ui::card_layer::CardAddress;

fn hand_transform(index: usize, hand_size: usize) -> (i32, i32) {
    let center = hand_size as i32 - 1;
    let distance = index as i32 * 2 - center;
    let translate_y = (distance.abs() * distance.abs() * 4) / 16;
    (distance, translate_y)
}

#[component]
pub fn Hand() -> Element {
    let state = use_context::<UIState>();
    let selected_card = state.selected_card;
    let hand_cards = (state.hand_contents)();
    let hand_size = hand_cards.len();

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[27.5%] z-50",
            for (index, card) in hand_cards.into_iter().enumerate() {
                {
                    let address = CardAddress {
                        controller: Player::Player,
                        location: CardLocation::Hand,
                        sequence: index as u32,
                    };
                    let is_selected = selected_card().is_some_and(|selected| selected.matches(address));
                    let (distance, default_translate_y) = hand_transform(index, hand_size);
                    let rotation = if is_selected { 0 } else { distance };
                    let translate_y = if is_selected { -25 } else { default_translate_y };
                    let stacking_order = if is_selected { hand_size } else { index };
                    rsx!(
                        div {
                            key: "{card.code}-{index}",
                            class: "w-[10vw] aspect-[59/86] shrink-0 -mx-[1.1vw] relative",
                            z_index: "{stacking_order}",
                            div {
                                class: "absolute inset-0 group [transition:transform_150ms_ease-in-out]",
                                transform: "translateZ(0) rotateZ({rotation}deg) translateY({translate_y}%)",
                                InteractiveCard {
                                    address,
                                    card: CardData {
                                        card_code: card.code,
                                        controller: Player::Player,
                                        location: CardLocation::Hand,
                                        ..CardData::default()
                                    },
                                }
                            }
                        }
                    )
                }
            }
        }
    )
}
