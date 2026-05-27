use dioxus::prelude::*;

use crate::ocgcore::HandCard;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::SelectedCard;
use crate::state::send_user_response;
use crate::ui::components::ActionButton;
use crate::ui::components::Card;
use crate::ui::components::CardActionMenu;
use crate::ui::components::svg::SummonIcon;

#[component]
pub fn Hand(
    cards: ReadSignal<Vec<HandCard>>,
    selected_card: WriteSignal<Option<SelectedCard>>,
) -> Element {
    let hand_size = cards().len() as i32;
    let center = hand_size - 1;

    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[27.5%] z-50",
            for (index, card) in cards().iter().copied().enumerate() {
                {
                    let is_selected = selected_card().is_some_and(|s| s.location == CardLocation::Hand && s.index == index);
                    let distance = (index as i32) * 2 - center;
                    let rotation = if is_selected { 0 } else { distance };
                    let translate_y = if is_selected { -25 } else { (distance.abs() * distance.abs() * 4) / 16 };

                    rsx!(
                        div {
                            key: "{card.code}-{index}",
                            id: index,
                            class: "transform-gpu transition duration-150 ease-in-out -mx-[1.1vw] relative group",
                            transform: "rotateZ({rotation}deg) translateY({translate_y}%)",
                            z_index: if is_selected { 100 } else { 0 },
                            CardActionMenu {
                                class: "absolute -top-28 left-1/2 transform -translate-x-1/2 flex flex-row items-center justify-center",
                                trigger: is_selected && (card.normal_summon_index.is_some() || card.is_activatable_or_chainable || card.spell_trap_set_index.is_some()),
                                if card.normal_summon_index.is_some() {
                                    ActionButton {
                                        label: "Summon",
                                        class: "border-cyan-500 text-cyan-300",
                                        onclick: move |evt: MouseEvent| {
                                            evt.stop_propagation();
                                            selected_card.set(None);
                                            if let Some(index) = card.normal_summon_index { send_user_response(UserResponse::NormalSummon { index }); }
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
                                            if let Some(index) = card.chain_index { send_user_response(UserResponse::Chain { index }); }
                                            if let Some(index) = card.activate_index { send_user_response(UserResponse::Activate { index }); }
                                        },
                                        SummonIcon {}
                                    }
                                }
                                if card.spell_trap_set_index.is_some() || card.monster_set_index.is_some() {
                                    ActionButton {
                                        label: "Set",
                                        class: "border-orange-500 text-orange-400",
                                        onclick: move |evt: MouseEvent| {
                                            evt.stop_propagation();
                                            selected_card.set(None);
                                            if let Some(index) = card.spell_trap_set_index { send_user_response(UserResponse::SetSpellTrap { index }); }
                                            if let Some(index) = card.monster_set_index { send_user_response(UserResponse::SetMonster { index }); }
                                        },
                                        SummonIcon { }
                                    }
                                }
                            }
                            Card {
                                code: card.code,
                                class: "w-[8vw]",
                                is_selected,
                                show_highlight_on_select: false,
                                show_dotted_highlight: false,
                                show_blue_aura: card.normal_summon_index.is_some() || card.spell_trap_set_index.is_some() || card.monster_set_index.is_some(),
                                show_orange_aura: card.is_activatable_or_chainable,
                                facedown: false,
                                use_extra_deck_back: false,
                                onclick:  move |evt: MouseEvent| {
                                    evt.stop_propagation();
                                    selected_card.set(Some(SelectedCard { location: CardLocation::Hand, index }));
                                },
                            }
                        }
                    )
                }
            }
        }
    )
}
