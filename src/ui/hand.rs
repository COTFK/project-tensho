use dioxus::prelude::*;

use super::components::ActionButton;
use super::components::Card;
use super::components::CardActionMenu;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::SelectedCard;
use crate::state::send_user_response;
use crate::ui::components::svg::SummonIcon;

#[component]
pub fn Hand() -> Element {
    let state = use_context::<DuelState>();
    let cards = state.hand_contents;

    let normal_summons = (state.normal_summons)();
    let activatable_effects = (state.activatable_effects)();
    let chainables = state.card_prompting_to_activate;
    let mut selected_card = state.selected_card;

    let hand_size = cards().len() as i32;
    let center = hand_size - 1;

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[27.5%] z-50",
            for (index, card) in cards().iter().copied().enumerate() {
                {
                    let normal_summon_index = normal_summons.get(&(index as u8)).copied().map(|v| v as u8);
                    let is_normal_summonable = normal_summon_index.is_some();

                    let activatable_index = activatable_effects
                        .iter()
                        .find(|(_activate_index, c)| c.location == CardLocation::Hand && c.sequence == index as u8)
                        .map(|(idx, _c)| *idx as u8);
                    let chainable_index = chainables
                        .iter()
                        .find(|c| c.location == CardLocation::Hand && c.sequence == index as u8)
                        .and_then(|c| c.chain_option)
                        .map(|v| v as u8);
                    let is_activatable = chainable_index.is_some() || activatable_index.is_some();

                    let is_selected = match selected_card() {
                        Some(sc) => sc.index == index as u8 && sc.location == CardLocation::Hand,
                        None => false,
                    };

                    let distance = (index as i32) * 2 - center;
                    let abs_distance = distance.abs();
                    let mut rotation = distance;
                    let mut translate_y = (abs_distance * abs_distance * 4) / 16;
                    if is_selected {
                        translate_y = -25;
                        rotation = 0;
                    }

                    let on_select = move |evt: MouseEvent| {
                        evt.stop_propagation();
                        selected_card.set(Some(SelectedCard{
                            location: CardLocation::Hand,
                            index: index as u8,
                        }));
                    };

                    let on_normal_summon = move |evt: MouseEvent| {
                        evt.stop_propagation();
                        if let Some(seq) = normal_summon_index { send_user_response(UserResponse::NormalSummon { sequence: seq }); }
                    };

                    let on_activate = move |evt: MouseEvent| {
                        evt.stop_propagation();
                        if let Some(seq) = chainable_index { send_user_response(UserResponse::Chain { sequence: seq }); }
                        if let Some(seq) = activatable_index { send_user_response(UserResponse::Activate { sequence: seq }); }
                    };

                    rsx!(
                        div {
                            key: "{card.unwrap().card_code}-{index}",
                            id: index,
                            class: "transform-gpu transition duration-150 ease-in-out -mx-[1.1vw] relative group",
                            transform: "rotateZ({rotation}deg) translateY({translate_y}%)",
                            z_index: if is_selected {100} else {0},
                            CardActionMenu {
                                class: "absolute -top-28 left-1/2 transform -translate-x-1/2",
                                trigger: is_selected && (is_normal_summonable || is_activatable),
                                if is_normal_summonable {
                                    ActionButton {
                                        label: "Summon",
                                        class: "border-cyan-500 text-cyan-300",
                                        onclick: on_normal_summon,
                                        SummonIcon { }
                                    }
                                }
                                if is_activatable {
                                    ActionButton {
                                        label: "Activate",
                                        class: "border-yellow-500 text-yellow-300",
                                        onclick: on_activate,
                                        SummonIcon {}
                                    }
                                }
                            }
                            Card {
                                code: card.unwrap().card_code,
                                class: "w-[8vw]",
                                is_selected,
                                highlight_on_select: false,
                                is_normal_summonable,
                                is_activatable,
                                onclick: on_select,
                            }
                        }
                    )
                }
            }
        }
    )
}
