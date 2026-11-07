use dioxus::prelude::*;

use super::card_interaction::InteractiveCard;
use super::layout_anchors::CardAnchor;
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ui::card_layer::{Card as TrackedCard, CardAddress, CardId, CardPlacement};
use crate::ui::components::CardImage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardAppearance {
    Interactive,
    Pile(CardImage),
    Material,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CardTarget {
    pub id: CardId,
    pub address: CardAddress,
    pub anchor: CardAnchor,
    pub data: CardData,
    pub rotation: f64,
    pub stack_order: i32,
    pub interactive: bool,
    pub appearance: CardAppearance,
}

fn visible_card(card: TrackedCard, target: Option<CardTarget>) -> Option<(CardTarget, bool)> {
    let target = target?;
    let address = match (card.placement, target.appearance) {
        (CardPlacement::Zone(address), CardAppearance::Interactive)
            if matches!(
                address.location,
                CardLocation::Hand | CardLocation::MonsterZone | CardLocation::SpellTrapZone
            ) =>
        {
            address
        }
        (CardPlacement::Zone(address), CardAppearance::Pile(_))
            if matches!(
                address.location,
                CardLocation::Deck
                    | CardLocation::ExtraDeck
                    | CardLocation::Graveyard
                    | CardLocation::Banishment
            ) =>
        {
            address
        }
        (CardPlacement::Overlay { .. }, CardAppearance::Material) => target.address,
        _ => return None,
    };
    if address.controller != Player::Player {
        return None;
    }
    if target.id != card.id
        || target.address != address
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
            && target.appearance == CardAppearance::Interactive
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
pub(super) fn PersistentCardLayer(cards: Vec<TrackedCard>, targets: Vec<CardTarget>) -> Element {
    let mut cards = cards;
    cards.sort_by_key(|card| {
        let target = targets.iter().find(|target| target.id == card.id);
        let order = target.map_or(0, |target| target.stack_order);
        let sequence = match (card.placement, target.map(|target| target.appearance)) {
            (CardPlacement::Zone(address), Some(CardAppearance::Pile(_))) => address.sequence,
            (CardPlacement::Overlay { sequence, .. }, Some(CardAppearance::Material)) => sequence,
            _ => 0,
        };
        (order, sequence)
    });
    rsx! {
        div {
            class: "absolute inset-0 pointer-events-none",
            for card in cards {
                LayerCard {
                    key: "{card.id}",
                    card,
                    target: targets.iter().find(|target| target.id == card.id).copied(),
                }
            }
        }
    }
}

#[component]
fn LayerCard(card: TrackedCard, target: Option<CardTarget>) -> Element {
    let visible = visible_card(card, target);
    let hand = matches!(card.placement, CardPlacement::Zone(address) if address.location == CardLocation::Hand);
    let style = visible.map(|(target, _)| {
        let offset = match (card.placement, target.appearance) {
            (CardPlacement::Zone(address), CardAppearance::Pile(_)) => (f64::from(address.sequence) + 1.0) * 0.01,
            _ => 0.0,
        };
        let (x_offset, y_offset) = if target.address.location == CardLocation::Banishment && matches!(target.appearance, CardAppearance::Pile(_)) {
            (format!("- {offset}vh"), format!("- {offset}vw"))
        } else {
            (format!("+ {offset}vw"), format!("- {offset}vh"))
        };
        format!(
            "--card-x: calc({}px {}); --card-y: calc({}px {}); --card-width: {}px; --card-height: {}px; --card-z: {};",
            target.anchor.center_x - target.anchor.width / 2.0, x_offset,
            target.anchor.center_y - target.anchor.height / 2.0, y_offset,
            target.anchor.width, target.anchor.height, target.stack_order,
        )
    }).unwrap_or_default();
    rsx! {
        div {
            "data-card-id": "{card.id}",
            class: "fixed left-0 top-0 pointer-events-none [transform:translate(var(--card-x),var(--card-y))] w-[var(--card-width)] h-[var(--card-height)] z-[var(--card-z)]",
            class: if hand { "[transition:transform_150ms_ease-in-out]" },
            class: if visible.is_none() { "hidden" },
            style,
            if let Some((target, facedown)) = visible {
                div {
                    class: "w-full h-full [transform:rotate(var(--card-rotation))]",
                    class: if hand { "[transition:transform_150ms_ease-in-out]" },
                    style: "--card-rotation: {target.rotation}deg;",
                    div {
                        class: "w-full h-full",
                        class: if target.interactive { "pointer-events-auto" } else { "pointer-events-none" },
                        InteractiveCard {
                            card: target.data,
                            address: target.address,
                            layered: true,
                            force_facedown: facedown,
                            passive: target.appearance != CardAppearance::Interactive,
                            image: match target.appearance {
                                CardAppearance::Pile(image) => Some(image),
                                _ => None,
                            },
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
