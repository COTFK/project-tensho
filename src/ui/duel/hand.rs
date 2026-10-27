use dioxus::prelude::*;

use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::Player;
use crate::state::SelectedCard;
use crate::state::UIState;
use crate::state::send_response;
use crate::ui::components::ActionButton;
use crate::ui::components::Card;
use crate::ui::components::CardActionMenu;
use crate::ui::components::svg::BoltShieldIcon;
use crate::ui::components::svg::LightningTrioIcon;
use crate::ui::components::svg::SummonIcon;

fn hand_transform(index: usize, hand_size: usize) -> (i32, i32) {
    let center = hand_size as i32 - 1;
    let distance = index as i32 * 2 - center;
    let translate_y = (distance.abs() * distance.abs() * 4) / 16;
    (distance, translate_y)
}

#[component]
pub fn Hand() -> Element {
    let state = use_context::<UIState>();
    let cards = state.hand_contents;
    let mut selected_card = state.selected_card;
    let suppress_actions = !state.effects_to_select_from.is_empty();
    let hand_cards = cards();
    let hand_size = hand_cards.len();

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[27.5%] z-50",
            for (index, card) in hand_cards.into_iter().enumerate() {
                {
                    let is_selected = selected_card().is_some_and(|s| s.location == CardLocation::Hand && s.index == index);
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
                                    CardActionMenu {
                                class: "absolute top-0 left-1/2 transform -translate-x-1/2 -translate-y-[110%] flex flex-row items-center justify-center px-10 py-1 md:px-12",
                                trigger: is_selected && (card.normal_summon_index.is_some() || card.is_activatable_or_chainable || card.spell_trap_set_index.is_some() || card.monster_set_index.is_some()) && !suppress_actions,
                                if card.normal_summon_index.is_some() {
                                    ActionButton {
                                        label: "Summon",
                                        class: "border-cyan-500 text-cyan-300",
                                        onclick: move |evt: MouseEvent| {
                                            evt.stop_propagation();
                                            selected_card.set(None);
                                            if let Some(index) = card.normal_summon_index { send_response(Response::NormalSummon { index }); }
                                        },
                                        SummonIcon { }
                                    }
                                }
                                if card.is_activatable_or_chainable {
                                    ActionButton {
                                        label: "Activate",
                                        class: "border-yellow-500 text-yellow-300",
                                        onclick: move |evt: MouseEvent| {
                                            evt.stop_propagation();
                                            selected_card.set(None);
                                            if let Some(index) = card.chain_index { send_response(Response::Chain { index }); }
                                            if let Some(index) = card.activate_index { send_response(Response::Activate { index }); }
                                        },
                                        LightningTrioIcon {}
                                    }
                                }
                                if card.spell_trap_set_index.is_some() || card.monster_set_index.is_some() {
                                    ActionButton {
                                        label: "Set",
                                        class: "border-orange-500 text-orange-400",
                                        onclick: move |evt: MouseEvent| {
                                            evt.stop_propagation();
                                            selected_card.set(None);
                                            if let Some(index) = card.spell_trap_set_index { send_response(Response::SetSpellTrap { index }); }
                                            if let Some(index) = card.monster_set_index { send_response(Response::SetMonster { index }); }
                                        },
                                        BoltShieldIcon {}
                                    }
                                }
                                    }
                                    Card {
                                card: CardData {
                                    card_code: card.code,
                                    controller: Player::Player,
                                    location: CardLocation::Hand,
                                    ..CardData::default()
                                },
                                class: "w-[10vw]",
                                is_selected,
                                show_highlight_on_select: false,
                                show_dotted_highlight: false,
                                show_blue_aura: (card.normal_summon_index.is_some() || card.spell_trap_set_index.is_some() || card.monster_set_index.is_some()) && !suppress_actions,
                                show_orange_aura: card.is_activatable_or_chainable && !suppress_actions,
                                facedown: false,
                                use_extra_deck_back: false,
                                onclick:  move |evt: MouseEvent| {
                                    evt.stop_propagation();
                                    selected_card.set(Some(SelectedCard { location: CardLocation::Hand, index }));
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
