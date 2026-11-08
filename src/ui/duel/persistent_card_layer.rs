use dioxus::prelude::*;
use std::collections::BTreeMap;

use super::card_interaction::CardWithActions;
use super::card_layout::{CardLayoutTarget, CardRenderMode};
use super::card_stats::CardStats;
use super::chain_badges::ChainBadges;
use crate::duel::cards::{CardInstanceId, CardPlacement, TrackedCard};
use crate::duel::controller::ChainLink;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Player};
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ui::components::CardImage;

fn visible_card_target(
    card: TrackedCard,
    target: Option<CardLayoutTarget>,
) -> Option<(CardLayoutTarget, bool)> {
    let target = target?;
    let address = match (card.placement, target.render_mode) {
        (CardPlacement::Zone(address), CardRenderMode::Interactive)
            if matches!(
                address.location,
                CardLocation::Hand | CardLocation::MonsterZone | CardLocation::SpellTrapZone
            ) =>
        {
            address
        }
        (CardPlacement::Zone(address), CardRenderMode::Pile(_))
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
        (CardPlacement::Overlay { .. }, CardRenderMode::Material) => target.address,
        _ => return None,
    };
    if address.controller != Player::Local {
        return None;
    }
    if !target.geometry.center_x.is_finite()
        || !target.geometry.center_y.is_finite()
        || !target.geometry.width.is_finite()
        || !target.geometry.height.is_finite()
        || target.geometry.width <= 0.0
        || target.geometry.height <= 0.0
        || !target.rotation.is_finite()
    {
        tracing::error!("Invalid card layer geometry for {:?}", card.id);
        return None;
    }
    let facedown = card.code == 0
        || (address.location != CardLocation::Hand
            && target.render_mode == CardRenderMode::Interactive
            && !card.position.is_some_and(|position| {
                position.intersects(
                    CardPositionFlags::FACE_UP_ATTACK | CardPositionFlags::FACE_UP_DEFENSE,
                ) && !position.intersects(
                    CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE,
                )
            }));
    Some((target, facedown))
}

pub(super) fn card_image(card: TrackedCard, target: Option<CardLayoutTarget>) -> Option<CardImage> {
    let (target, facedown) = visible_card_target(card, target)?;
    Some(match target.render_mode {
        CardRenderMode::Pile(image) => image,
        _ if facedown
            || (target.address.location != CardLocation::Hand
                && matches!(
                    target.data.position,
                    Some(
                        BattlePosition::FaceDown
                            | BattlePosition::FaceDownAttack
                            | BattlePosition::FaceDownDefense
                    )
                )) =>
        {
            CardImage::CardBack
        }
        _ => CardImage::Face(target.data),
    })
}

fn chain_indicators(
    cards: &[TrackedCard],
    links: &[ChainLink],
) -> BTreeMap<CardInstanceId, Vec<u32>> {
    let mut indicators = BTreeMap::<CardInstanceId, Vec<u32>>::new();
    for link in links {
        let Some(card) = link
            .card
            .and_then(|id| cards.iter().find(|card| card.id == id))
        else {
            continue;
        };
        let card = match card.placement {
            CardPlacement::Overlay { host, .. } => cards.iter().find(|card| card.id == host),
            CardPlacement::Zone(_) => Some(card),
        };
        let Some(card) = card else { continue };
        let CardPlacement::Zone(address) = card.placement else {
            continue;
        };
        if address.controller != Player::Local {
            continue;
        }
        let target = match address.location {
            CardLocation::Hand | CardLocation::MonsterZone | CardLocation::SpellTrapZone => card,
            CardLocation::Deck
            | CardLocation::ExtraDeck
            | CardLocation::Graveyard
            | CardLocation::Banishment => cards
                .iter()
                .filter(|candidate| {
                    matches!(candidate.placement,
                        CardPlacement::Zone(other) if other.controller == address.controller
                            && other.location == address.location
                    )
                })
                .max_by_key(|candidate| match candidate.placement {
                    CardPlacement::Zone(other) => other.sequence,
                    CardPlacement::Overlay { .. } => unreachable!(),
                })
                .expect("Chain source belongs to its pile"),
        };
        indicators.entry(target.id).or_default().push(link.number);
    }
    indicators
}

#[component]
pub(super) fn PersistentCardLayer(
    cards: Vec<TrackedCard>,
    targets: Vec<CardLayoutTarget>,
    selected: Option<CardInstanceId>,
    suppressed: bool,
    animating: bool,
    moving: Vec<CardInstanceId>,
    chain_links: Vec<ChainLink>,
) -> Element {
    let indicators = chain_indicators(&cards, &chain_links);
    let mut cards = cards;
    cards.sort_by_key(|card| card.id);
    rsx! {
        div {
            class: "absolute inset-0 isolate pointer-events-none",
            for card in cards {
                LayerCard {
                    key: "{card.id}",
                    card,
                    target: targets.iter().find(|target| target.id == card.id).copied(),
                    chain_links: indicators.get(&card.id).cloned().unwrap_or_default(),
                    selected: selected == Some(card.id),
                    suppressed,
                    animating,
                    moving: moving.contains(&card.id),
                }
            }
        }
    }
}

#[component]
fn LayerCard(
    card: TrackedCard,
    target: Option<CardLayoutTarget>,
    chain_links: Vec<u32>,
    selected: bool,
    suppressed: bool,
    animating: bool,
    moving: bool,
) -> Element {
    let visible = target.zip(card_image(card, target));
    let hand = matches!(card.placement, CardPlacement::Zone(address) if address.location == CardLocation::Hand);
    let style = visible.map(|(target, _)| {
        let offset = match (card.placement, target.render_mode) {
            (CardPlacement::Zone(address), CardRenderMode::Pile(_)) => (f64::from(address.sequence) + 1.0) * 0.01,
            _ => 0.0,
        };
        let (x_offset, y_offset) = if target.address.location == CardLocation::Banishment && matches!(target.render_mode, CardRenderMode::Pile(_)) {
            (format!("- {offset}vh"), format!("- {offset}vw"))
        } else {
            (format!("+ {offset}vw"), format!("- {offset}vh"))
        };
        let stack = match (card.placement, target.render_mode) {
            (CardPlacement::Zone(address), CardRenderMode::Pile(_)) => i64::from(address.sequence) + 1,
            (CardPlacement::Overlay { sequence, .. }, CardRenderMode::Material) => i64::from(sequence) + 1,
            (_, CardRenderMode::Interactive) if hand => 1000 + i64::from(target.stack_order),
            (_, CardRenderMode::Interactive) => 500,
            _ => i64::from(target.stack_order),
        };
        format!(
            "--card-x: calc({}px {}); --card-y: calc({}px {}); --card-width: {}px; --card-height: {}px; --card-z: {};",
            target.geometry.center_x - target.geometry.width / 2.0, x_offset,
            target.geometry.center_y - target.geometry.height / 2.0, y_offset,
            target.geometry.width, target.geometry.height,
            if moving { 2000 + stack } else if selected && !hand && target.render_mode == CardRenderMode::Interactive { 4000 + stack } else { stack },
        )
    }).unwrap_or_default();
    rsx! {
        div {
            "data-card-id": "{card.id}",
            class: "fixed left-0 top-0 pointer-events-none [transform:translate(var(--card-x),var(--card-y))] w-[var(--card-width)] h-[var(--card-height)] z-[var(--card-z)]",
            class: if animating { "[transition:transform_300ms_ease-in-out,width_300ms_ease-in-out,height_300ms_ease-in-out]" } else if hand { "[transition:transform_150ms_ease-in-out]" },
            class: if visible.is_none() { "hidden" },
            style,
            if let Some((target, image)) = visible {
                div {
                    class: "relative w-full h-full [transform:rotate(var(--card-rotation))]",
                    class: if animating { "[transition:transform_300ms_ease-in-out]" } else if hand { "[transition:transform_150ms_ease-in-out]" },
                    style: "--card-rotation: {target.rotation}deg;",
                    div {
                        class: "w-full h-full",
                        class: if target.render_mode == CardRenderMode::Interactive { "pointer-events-auto" } else { "pointer-events-none" },
                        CardWithActions {
                            card: target.data,
                            address: target.address,
                            passive: target.render_mode != CardRenderMode::Interactive,
                            animated: animating,
                            image,
                            selected,
                            suppress: suppressed,
                        }
                    }
                    ChainBadges { links: chain_links, rotation: target.rotation }
                    if !moving && target.render_mode == CardRenderMode::Interactive && matches!(target.address.location, CardLocation::MonsterZone | CardLocation::SpellTrapZone) {
                        div {
                            "data-card-stats": "{card.id}",
                            class: "absolute top-0 left-1/2 -translate-x-1/2 h-full w-[var(--card-height)] pointer-events-none",
                            CardStats { card: target.data }
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
