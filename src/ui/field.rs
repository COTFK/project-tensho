use dioxus::prelude::*;

use super::components::svg::SummonIcon;
use super::extra_deck::ExtraDeck;
use super::graveyard::Graveyard;
use super::main_deck::MainDeck;
use crate::ocgcore::CardData;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::SelectedCard;
use crate::state::send_user_response;
use crate::ui::components::ActionButton;
use crate::ui::components::Card;
use crate::ui::components::CardActionMenu;

#[component]
pub fn Field() -> Element {
    let state = use_context::<DuelState>();
    let monsters = (state.monsters)();
    let spell_traps = (state.spell_traps)();

    rsx!(
        div {
            class: "pt-[15vh] max-w-[70vw] w-fit mx-auto grid grid-cols-7 gap-0.5 justify-items-center",

            // Extra monster zones (row 1)
            div { class: "col-start-3 row-start-1", Zone { index: 5, card: monsters.get(5).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-1", Zone { index: 6, card: monsters.get(6).copied().flatten(), location: CardLocation::MonsterZone } }

            // Main row (row 2)
            div { class: "col-start-1 row-start-2", Zone { index: 5, card: spell_traps.get(5).copied().flatten(), location: CardLocation::SpellTrapZone } }

            div { class: "col-start-2 row-start-2" , Zone { index: 0, card: monsters.first().copied().flatten(), location: CardLocation::MonsterZone} }
            div { class: "col-start-3 row-start-2" , Zone { index: 1, card: monsters.get(1).copied().flatten(), location: CardLocation::MonsterZone} }
            div { class: "col-start-4 row-start-2" , Zone { index: 2, card: monsters.get(2).copied().flatten(), location: CardLocation::MonsterZone} }
            div { class: "col-start-5 row-start-2" , Zone { index: 3, card: monsters.get(3).copied().flatten(), location: CardLocation::MonsterZone} }
            div { class: "col-start-6 row-start-2" , Zone { index: 4, card: monsters.get(4).copied().flatten(), location: CardLocation::MonsterZone} }

            div { class: "col-start-7 row-start-2", Graveyard {} }

            // Spell/Trap row (row 3)
            div { class: "col-start-1 row-start-3", ExtraDeck {} }

            div { class: "col-start-2 row-start-3", Zone { index: 0, card: spell_traps.first().copied().flatten(), location: CardLocation::SpellTrapZone} }
            div { class: "col-start-3 row-start-3", Zone { index: 1, card: spell_traps.get(1).copied().flatten(), location: CardLocation::SpellTrapZone} }
            div { class: "col-start-4 row-start-3", Zone { index: 2, card: spell_traps.get(2).copied().flatten(), location: CardLocation::SpellTrapZone} }
            div { class: "col-start-5 row-start-3", Zone { index: 3, card: spell_traps.get(3).copied().flatten(), location: CardLocation::SpellTrapZone} }
            div { class: "col-start-6 row-start-3", Zone { index: 4, card: spell_traps.get(4).copied().flatten(), location: CardLocation::SpellTrapZone} }

            div { class: "col-start-7 row-start-3", MainDeck {} }
        }
    )
}

#[component]
fn Zone(index: u8, location: CardLocation, card: Option<CardData>) -> Element {
    let state = use_context::<DuelState>();

    let placeable_on = (state.available_zones)()
        .iter()
        .any(|zone| zone.0 == location && zone.1 == index);

    rsx!(
        div {
            class: "bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center",
            class: if placeable_on {"border-2 border-yellow-300"} else {"border-0.5"},
            onclick: move |_| {
                if placeable_on {
                    send_user_response(UserResponse::Place {
                        controller: CardController::Player as u8,
                        location: location as u8,
                        index: index as u8,
                    });
                }
            },
            if let Some(card) = card {
                FieldCard {
                    index: index,
                    location: location,
                    card
                }
            }
        }
    )
}

#[component]
pub fn FieldCard(index: u8, location: CardLocation, card: CardData) -> Element {
    let mut state = use_context::<DuelState>();

    let mut selected_card = state.selected_card;
    let activatable_map = (state.activatable_effects)();
    let prompt_list = (state.card_prompting_to_activate)();
    let selected_snapshot = selected_card();
    let effects_of_this_card: Vec<CardData> = activatable_map
        .iter()
        .filter(|card| card.location == location && card.sequence == index)
        .copied()
        .collect();

    let activatable_eff_index = effects_of_this_card
        .first()
        .and_then(|card| card.action_index);

    let prompted_card = prompt_list
        .iter()
        .find(|card| card.location == location && card.sequence == index);

    let prompted = prompted_card.is_some();
    let activatable = !effects_of_this_card.is_empty();
    let chain_index = prompted_card.and_then(|card| card.action_index);

    let is_selected = selected_snapshot
        .is_some_and(|card| card.location == location && card.index as u8 == index);

    let cards_to_select_from = (state.cards_to_select_from)();
    let card_in_select_list = cards_to_select_from
        .as_ref()
        .and_then(|message| message.select_card_for(location, index));
    let select_unselect_index = cards_to_select_from.as_ref().and_then(|message| {
        card_in_select_list.and_then(|selectable_card| message.response_index_for(&selectable_card))
    });

    let selectable_for_extra_deck_summon = card_in_select_list.is_some();
    let is_selected_for_extra_deck_summon = if selectable_for_extra_deck_summon {
        card_in_select_list.unwrap().is_selected
    } else {
        false
    };

    rsx!(
        div {
            class: "relative h-full aspect-[59/86] mx-auto p-[clamp(1px,0.3vw,5px)]",
            CardActionMenu {
                class: "absolute -top-28 left-1/2 transform -translate-x-1/2",
                trigger: is_selected && (prompted || activatable),
                if prompted || activatable {
                    ActionButton {
                        label: "Activate",
                        class: "border-yellow-500 text-yellow-300",
                        onclick: move |evt: MouseEvent| {
                            evt.stop_propagation();

                            if prompted {
                                if let Some(index) = chain_index {
                                    send_user_response(UserResponse::Chain { index });
                                } else {
                                    send_user_response(UserResponse::Yes);
                                }
                            }

                            if activatable {
                                if effects_of_this_card.len() > 1 {
                                    state.effects_to_select_from.set(effects_of_this_card.to_owned());
                                    state.selected_card.set(None);
                                } else if let Some(idx) = activatable_eff_index {
                                    send_user_response(UserResponse::Activate { index: idx });
                                }
                            }
                        },
                        SummonIcon {}
                    }
                }
            }
            Card {
                code: card.card_code,
                class: if card.position == Some(BattlePosition::FaceDownDefense) || card.position == Some(BattlePosition::FaceUpDefense) { "-rotate-90" } else {""},
                is_selected: is_selected || is_selected_for_extra_deck_summon,
                show_highlight_on_select: true,
                show_blue_aura: false,
                show_dotted_highlight: selectable_for_extra_deck_summon,
                show_orange_aura: activatable || prompted,
                facedown: card.position == Some(BattlePosition::FaceDown) || card.position == Some(BattlePosition::FaceDownAttack) || card.position == Some(BattlePosition::FaceDownDefense),
                use_extra_deck_back: false,
                onclick: move |evt: MouseEvent| {
                    evt.stop_propagation();

                    if let Some(index) = select_unselect_index {
                        send_user_response(UserResponse::SelectUnselectCard { index });
                    } else {
                        selected_card.set(Some(SelectedCard {
                            location,
                            index: index as usize
                        }));
                    }
                },
            }
        }
    )
}
