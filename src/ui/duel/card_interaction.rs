use dioxus::prelude::*;

use crate::duel::input::{CardActions, PendingInput};
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{BattlePosition, CardLocation};
use crate::state::{UIState, select_card, send_response};
use crate::ui::card_layer::CardAddress;
use crate::ui::components::svg::{BoltShieldIcon, LightningTrioIcon, SummonIcon};
use crate::ui::components::{ActionButton, Card, CardActionMenu, CardImage};
use crate::ui::view_state::ViewState;

fn prompt_actions(state: &UIState, address: CardAddress) -> CardActions {
    state
        .pending_input
        .read()
        .as_ref()
        .map(|input| input.card_actions(address))
        .unwrap_or_default()
}

fn current_menu_actions(
    state: &UIState,
    view: &ViewState,
    address: CardAddress,
) -> Option<CardActions> {
    if !view.is_selected(address, state.cards.peek().as_ref())
        || !view.effects_to_select_from.peek().is_empty()
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
    #[props(default)] force_facedown: bool,
    #[props(default)] passive: bool,
    #[props(default)] animated: bool,
    #[props(default)] image: Option<CardImage>,
) -> Element {
    let mut state = use_context::<UIState>();
    let mut view = use_context::<ViewState>();
    let actions = if passive {
        CardActions::default()
    } else {
        prompt_actions(&state, address)
    };
    let hand = address.location == CardLocation::Hand;
    let selected = !passive && view.is_selected(address, state.cards.read().as_ref());
    let suppress = !view.effects_to_select_from.is_empty();
    let prompted = actions.prompted;
    let effects = &actions.effects;
    let can_activate = if hand {
        prompted.and_then(|card| card.action_index).is_some() || !effects.is_empty()
    } else {
        prompted.is_some() || !effects.is_empty()
    };
    let can_set = actions.monster_set.is_some() || actions.spell_trap_set.is_some();
    let menu_class = if hand {
        "absolute top-0 left-1/2 transform -translate-x-1/2 -translate-y-[110%] flex flex-row items-center justify-center px-10 py-1 md:px-12 z-20"
    } else {
        "absolute left-1/2 bottom-1/2 -translate-x-[50%] translate-y-[50%] px-3 py-1 z-20"
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
            class: if passive || !hand { "relative w-full h-full" } else { "" },
            CardActionMenu {
                class: menu_class,
                trigger: selected && !suppress && (can_activate || (hand && (actions.summon.is_some() || can_set))),
                if hand && actions.summon.is_some() {
                    ActionButton {
                        label: "Summon",
                        class: "border-cyan-500 text-cyan-300",
                        onclick: move |event: MouseEvent| {
                            event.stop_propagation();
                            let Some(actions) = current_menu_actions(&state, &view, address) else { return };
                            let Some(index) = actions.summon else {
                                tracing::warn!("Summon is no longer available at {address:?}");
                                return;
                            };
                            view.selected_card.set(None);
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
                            let Some(actions) = current_menu_actions(&state, &view, address) else { return };
                            if let Some(prompted) = actions.prompted {
                                if let Some(index) = prompted.action_index {
                                    view.selected_card.set(None);
                                    send_response(Response::Chain { index });
                                    return;
                                }
                                if !hand {
                                    send_response(Response::Yes);
                                    return;
                                }
                            }
                            if !hand && actions.effects.len() > 1 {
                                view.effects_to_select_from.set(actions.effects);
                                view.selected_card.set(None);
                            } else if let Some(index) = actions.effects.last().and_then(|effect| effect.action_index) {
                                view.selected_card.set(None);
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
                            let Some(actions) = current_menu_actions(&state, &view, address) else { return };
                            if let Some(index) = actions.spell_trap_set {
                                view.selected_card.set(None);
                                send_response(Response::SetSpellTrap { index });
                            } else if let Some(index) = actions.monster_set {
                                view.selected_card.set(None);
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
                            state.pending_input.with_mut(|input| {
                                let Some(PendingInput::Tribute { selected, .. }) = input else {
                                    tracing::warn!("Tribute selection is no longer available at {address:?}");
                                    return;
                                };
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
