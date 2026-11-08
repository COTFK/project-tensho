use dioxus::prelude::*;

use crate::duel::cards::CardAddress;
use crate::duel::input::{CardAction, CardActions, PendingInput};
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{BattlePosition, CardLocation};
use crate::ui::components::svg::{BoltShieldIcon, LightningTrioIcon, SummonIcon};
use crate::ui::components::{ActionButton, Card, CardActionMenu, CardImage};

#[component]
pub(super) fn InteractiveCard(
    card: CardData,
    address: CardAddress,
    #[props(default)] force_facedown: bool,
    #[props(default)] passive: bool,
    #[props(default)] animated: bool,
    #[props(default)] image: Option<CardImage>,
    selected: bool,
    suppress: bool,
) -> Element {
    let pending = use_context::<ReadSignal<Option<PendingInput>>>();
    let onaction = use_context::<EventHandler<CardAction>>();
    let actions = if passive {
        CardActions::default()
    } else {
        pending
            .read()
            .as_ref()
            .map(|input| input.card_actions(address))
            .unwrap_or_default()
    };
    let hand = address.location == CardLocation::Hand;
    let selected = !passive && selected;
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
                            onaction.call(CardAction::Summon(address));
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
                            onaction.call(CardAction::Activate(address));
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
                            onaction.call(CardAction::Set(address));
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
                    onaction.call(CardAction::Select(address));
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/card_interaction.rs"]
mod tests;
