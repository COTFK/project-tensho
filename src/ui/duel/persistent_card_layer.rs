use dioxus::prelude::*;

use super::card_interaction::InteractiveCard;
use super::layout_anchors::CardAnchor;
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ui::card_layer::{Card as TrackedCard, CardId, CardPlacement};
use crate::ui::components::Card;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CardTarget {
    pub id: CardId,
    pub anchor: CardAnchor,
    pub data: CardData,
    pub rotation: f64,
    pub stack_order: i32,
    pub interactive: bool,
}

fn visible_card(card: TrackedCard, target: Option<CardTarget>) -> Option<(CardTarget, bool)> {
    let target = target?;
    let CardPlacement::Zone(address) = card.placement else {
        return None;
    };
    if address.controller != Player::Player
        || !matches!(
            address.location,
            CardLocation::Hand | CardLocation::MonsterZone | CardLocation::SpellTrapZone
        )
    {
        return None;
    }
    if target.id != card.id
        || target.data.controller != address.controller
        || target.data.location != address.location
        || u32::from(target.data.sequence) != address.sequence
        || (card.code != 0 && target.data.card_code != card.code)
    {
        tracing::error!(
            "Card layer target does not match tracked card {:?}",
            card.id
        );
        return None;
    }
    if !target.anchor.center_x.is_finite()
        || !target.anchor.center_y.is_finite()
        || !target.anchor.width.is_finite()
        || !target.anchor.height.is_finite()
        || target.anchor.width <= 0.0
        || target.anchor.height <= 0.0
        || !target.rotation.is_finite()
    {
        tracing::error!("Invalid card layer geometry for {:?}", card.id);
        return None;
    }
    let facedown = card.code == 0
        || (address.location != CardLocation::Hand
            && !card.position.is_some_and(|position| {
                position.intersects(
                    CardPositionFlags::FACE_UP_ATTACK | CardPositionFlags::FACE_UP_DEFENSE,
                ) && !position.intersects(
                    CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE,
                )
            }));
    Some((target, facedown))
}

#[component]
pub(super) fn PersistentCardLayer(
    cards: Vec<TrackedCard>,
    targets: Vec<CardTarget>,
    on_card_click: EventHandler<CardId>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 pointer-events-none",
            for card in cards {
                LayerCard {
                    key: "{card.id}",
                    card,
                    target: targets.iter().find(|target| target.id == card.id).copied(),
                    on_card_click,
                }
            }
        }
    }
}

#[component]
fn LayerCard(
    card: TrackedCard,
    target: Option<CardTarget>,
    on_card_click: EventHandler<CardId>,
) -> Element {
    let visible = visible_card(card, target);
    let style = visible.map(|(target, _)| format!(
        "--card-x: {}px; --card-y: {}px; --card-width: {}px; --card-height: {}px; --card-z: {};",
        target.anchor.center_x - target.anchor.width / 2.0,
        target.anchor.center_y - target.anchor.height / 2.0,
        target.anchor.width,
        target.anchor.height,
        target.stack_order,
    )).unwrap_or_default();
    rsx! {
        div {
            "data-card-id": "{card.id}",
            class: "absolute left-0 top-0 pointer-events-none [transform:translate(var(--card-x),var(--card-y))] w-[var(--card-width)] h-[var(--card-height)] z-[var(--card-z)]",
            class: if visible.is_none() { "hidden" },
            style,
            if let Some((target, facedown)) = visible {
                div {
                    class: "w-full h-full [transform:rotate(var(--card-rotation))]",
                    style: "--card-rotation: {target.rotation}deg;",
                    div {
                        class: "w-full h-full",
                        class: if target.interactive { "pointer-events-auto" } else { "pointer-events-none" },
                        if let CardPlacement::Zone(address) = card.placement {
                            if address.location == CardLocation::Hand {
                                Card {
                                    card: target.data,
                                    class: "w-full",
                                    is_selected: false,
                                    show_highlight_on_select: false,
                                    show_dotted_highlight: false,
                                    show_blue_aura: false,
                                    show_orange_aura: false,
                                    facedown,
                                    use_extra_deck_back: false,
                                    onclick: move |event: MouseEvent| {
                                        event.stop_propagation();
                                        on_card_click.call(card.id);
                                    },
                                }
                            } else {
                                InteractiveCard { card: target.data, address, layered: true, force_facedown: facedown }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/persistent_card_layer.rs"]
mod tests;
