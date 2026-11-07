use dioxus::prelude::*;

use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{BattlePosition, CardLocation};
use crate::ocgcore::messages::{SelectTributeMessageData, SelectUnselectMessageData};
use crate::state::{UIState, select_card, send_response};
use crate::ui::card_layer::CardAddress;
use crate::ui::components::svg::{BoltShieldIcon, LightningTrioIcon, SummonIcon};
use crate::ui::components::{ActionButton, Card, CardActionMenu, CardImage};

struct PromptCards<'a> {
    normal_summons: &'a [CardData],
    monster_sets: &'a [CardData],
    spell_trap_sets: &'a [CardData],
    activatable_effects: &'a [CardData],
    prompted_effects: &'a [CardData],
    selections: Option<&'a SelectUnselectMessageData>,
    tributes: Option<&'a SelectTributeMessageData>,
    selected_tributes: &'a [u8],
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CardActions {
    summon: Option<u8>,
    monster_set: Option<u8>,
    spell_trap_set: Option<u8>,
    effects: Vec<CardData>,
    prompted: Option<CardData>,
    selection: Option<u32>,
    selectable: bool,
    selected: bool,
    tribute: Option<u8>,
    tribute_selectable: bool,
    tribute_selected: bool,
    tribute_max: usize,
}

impl PromptCards<'_> {
    fn for_address(&self, address: CardAddress) -> CardActions {
        let matches = |card: &&CardData| {
            card.controller == address.controller
                && card.location == address.location
                && u32::from(card.sequence) == address.sequence
        };
        let last_index = |cards: &[CardData]| {
            cards
                .iter()
                .filter(matches)
                .last()
                .and_then(|card| card.action_index)
        };
        let selectable = self.selections.and_then(|message| {
            message.select_card_for(address.controller, address.location, address.sequence)
        });
        let tribute = self
            .tributes
            .and_then(|message| message.candidates.iter().find(matches).copied());
        let tribute_index = tribute.and_then(|card| card.action_index);
        CardActions {
            summon: last_index(self.normal_summons),
            monster_set: last_index(self.monster_sets),
            spell_trap_set: last_index(self.spell_trap_sets),
            effects: self
                .activatable_effects
                .iter()
                .filter(matches)
                .copied()
                .collect(),
            prompted: if address.location == CardLocation::Hand {
                self.prompted_effects.iter().filter(matches).last().copied()
            } else {
                self.prompted_effects.iter().find(matches).copied()
            },
            selection: self
                .selections
                .and_then(|message| selectable.and_then(|card| message.response_index_for(&card))),
            selectable: selectable.is_some(),
            selected: selectable.is_some_and(|card| card.is_selected),
            tribute: tribute_index,
            tribute_selectable: tribute.is_some(),
            tribute_selected: tribute_index
                .is_some_and(|index| self.selected_tributes.contains(&index)),
            tribute_max: self
                .tributes
                .map_or(0, |message| message.max_select as usize),
        }
    }
}

fn prompt_actions(state: &UIState, address: CardAddress) -> CardActions {
    let normal_summons = (state.normal_summons)();
    let monster_sets = (state.monster_sets)();
    let spell_trap_sets = (state.spell_trap_sets)();
    let activatable_effects = (state.activatable_effects)();
    let prompted_effects = (state.card_prompting_to_activate)();
    let selections = (state.cards_to_select_from)();
    let tributes = (state.tributes)();
    let selected_tributes = (state.selected_tributes)();
    PromptCards {
        normal_summons: &normal_summons,
        monster_sets: &monster_sets,
        spell_trap_sets: &spell_trap_sets,
        activatable_effects: &activatable_effects,
        prompted_effects: &prompted_effects,
        selections: selections.as_ref(),
        tributes: tributes.as_ref(),
        selected_tributes: &selected_tributes,
    }
    .for_address(address)
}

fn current_menu_actions(state: &UIState, address: CardAddress) -> Option<CardActions> {
    if !state
        .selected_card
        .peek()
        .is_some_and(|selected| selected.matches(address))
        || !state.effects_to_select_from.peek().is_empty()
    {
        tracing::warn!("Ignoring a stale or suppressed card action at {address:?}");
        return None;
    }
    Some(prompt_actions(state, address))
}

#[component]
pub(super) fn InteractiveCard(
    card: CardData,
    address: CardAddress,
    #[props(default)] layered: bool,
    #[props(default)] force_facedown: bool,
    #[props(default)] passive: bool,
    #[props(default)] animated: bool,
    #[props(default)] image: Option<CardImage>,
) -> Element {
    let mut state = use_context::<UIState>();
    let actions = if passive {
        CardActions::default()
    } else {
        prompt_actions(&state, address)
    };
    let hand = address.location == CardLocation::Hand;
    let selected =
        !passive && (state.selected_card)().is_some_and(|selected| selected.matches(address));
    let suppress = !state.effects_to_select_from.is_empty();
    let prompted = actions.prompted;
    let effects = &actions.effects;
    let can_activate = if hand {
        prompted.and_then(|card| card.action_index).is_some() || !effects.is_empty()
    } else {
        prompted.is_some() || !effects.is_empty()
    };
    let can_set = actions.monster_set.is_some() || actions.spell_trap_set.is_some();
    let menu_class = if hand {
        "absolute top-0 left-1/2 transform -translate-x-1/2 -translate-y-[110%] flex flex-row items-center justify-center px-10 py-1 md:px-12"
    } else {
        "absolute left-1/2 bottom-1/2 -translate-x-[50%] translate-y-[50%] px-3 py-1 z-10"
    };
    let card_class = if passive {
        "w-full"
    } else if hand {
        "w-[10vw]"
    } else if matches!(
        card.position,
        Some(BattlePosition::FaceDownDefense | BattlePosition::FaceUpDefense)
    ) {
        "-rotate-90"
    } else {
        ""
    };
    let facedown = force_facedown
        || !hand
            && matches!(
                card.position,
                Some(
                    BattlePosition::FaceDown
                        | BattlePosition::FaceDownAttack
                        | BattlePosition::FaceDownDefense
                )
            );
    rsx! {
        div {
            class: if passive || (layered && !hand) { "relative w-full h-full" } else if hand { "" } else { "relative h-full aspect-[59/86] mx-auto p-[clamp(1px,0.3vw,5px)]" },
            CardActionMenu {
                class: menu_class,
                trigger: selected && !suppress && (can_activate || (hand && (actions.summon.is_some() || can_set))),
                if hand && actions.summon.is_some() {
                    ActionButton {
                        label: "Summon",
                        class: "border-cyan-500 text-cyan-300",
                        onclick: move |event: MouseEvent| {
                            event.stop_propagation();
                            let Some(actions) = current_menu_actions(&state, address) else { return };
                            let Some(index) = actions.summon else {
                                tracing::warn!("Summon is no longer available at {address:?}");
                                return;
                            };
                            state.selected_card.set(None);
                            send_response(Response::NormalSummon { index });
                        },
                        SummonIcon {}
                    }
                }
                if can_activate {
                    ActionButton {
                        label: "Activate",
                        class: "border-yellow-500 text-yellow-300",
                        onclick: move |event: MouseEvent| {
                            event.stop_propagation();
                            let Some(actions) = current_menu_actions(&state, address) else { return };
                            if let Some(prompted) = actions.prompted {
                                if let Some(index) = prompted.action_index {
                                    state.selected_card.set(None);
                                    send_response(Response::Chain { index });
                                    return;
                                }
                                if !hand {
                                    send_response(Response::Yes);
                                    return;
                                }
                            }
                            if !hand && actions.effects.len() > 1 {
                                state.effects_to_select_from.set(actions.effects);
                                state.selected_card.set(None);
                            } else if let Some(index) = actions.effects.last().and_then(|effect| effect.action_index) {
                                state.selected_card.set(None);
                                send_response(Response::Activate { index });
                            } else {
                                tracing::warn!("Activation is no longer available at {address:?}");
                            }
                        },
                        LightningTrioIcon {}
                    }
                }
                if hand && can_set {
                    ActionButton {
                        label: "Set",
                        class: "border-orange-500 text-orange-400",
                        onclick: move |event: MouseEvent| {
                            event.stop_propagation();
                            let Some(actions) = current_menu_actions(&state, address) else { return };
                            if let Some(index) = actions.spell_trap_set {
                                state.selected_card.set(None);
                                send_response(Response::SetSpellTrap { index });
                            } else if let Some(index) = actions.monster_set {
                                state.selected_card.set(None);
                                send_response(Response::SetMonster { index });
                            } else {
                                tracing::warn!("Setting is no longer available at {address:?}");
                            }
                        },
                        BoltShieldIcon {}
                    }
                }
            }
            Card {
                image: image.unwrap_or(if facedown { CardImage::CardBack } else { CardImage::Face(card) }),
                animated,
                class: card_class,
                is_selected: selected || (!hand && (actions.selected || actions.tribute_selected)),
                show_highlight_on_select: !hand && !passive,
                show_blue_aura: hand && (actions.summon.is_some() || can_set) && !suppress,
                show_dotted_highlight: !hand && (actions.selectable || actions.tribute_selectable),
                show_orange_aura: can_activate && !suppress,
                onclick: move |event: MouseEvent| {
                    event.stop_propagation();
                    if passive { return; }
                    let actions = prompt_actions(&state, address);
                    if !hand {
                        if let Some(index) = actions.tribute {
                            state.selected_tributes.with_mut(|selected| {
                                if let Some(position) = selected.iter().position(|selected| *selected == index) {
                                    selected.remove(position);
                                } else if selected.len() < actions.tribute_max {
                                    selected.push(index);
                                }
                            });
                            return;
                        }
                        if let Some(index) = actions.selection {
                            send_response(Response::SelectUnselectCard { index });
                            return;
                        }
                    }
                    select_card(address);
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/card_interaction.rs"]
mod tests;
