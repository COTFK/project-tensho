use anyhow::{Context, ensure};
use dioxus::prelude::*;

use super::field::Field;
use super::layout_anchors::{CardAnchor, LayoutAnchors, LayoutRevision};
use super::persistent_card_layer::{CardTarget, PersistentCardLayer};
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::state::UIState;
use crate::ui::card_layer::{CardAddress, CardLayer, CardPlacement};
use crate::ui::card_tracking::CardTracking;

#[derive(Clone, PartialEq)]
struct PreviewSnapshot {
    cards: CardLayer,
    monsters: Vec<Option<CardData>>,
    spell_traps: Vec<Option<CardData>>,
}

fn snapshot(
    enabled: bool,
    tracking: Option<CardTracking>,
    monsters: Vec<Option<CardData>>,
    spell_traps: Vec<Option<CardData>>,
) -> Option<PreviewSnapshot> {
    match (enabled, tracking) {
        (true, Some(CardTracking::Active(cards))) => Some(PreviewSnapshot {
            cards,
            monsters,
            spell_traps,
        }),
        _ => None,
    }
}

async fn measure_field(anchors: &LayoutAnchors) -> anyhow::Result<Vec<CardAnchor>> {
    let mut result = Vec::with_capacity(13);
    for (location, count) in [
        (CardLocation::MonsterZone, 7),
        (CardLocation::SpellTrapZone, 6),
    ] {
        for sequence in 0..count {
            let address = CardAddress {
                controller: Player::Player,
                location,
                sequence,
            };
            result.push(
                anchors
                    .measure(address)
                    .await
                    .with_context(|| format!("Cannot measure {address:?}"))?,
            );
        }
    }
    Ok(result)
}

fn targets(snapshot: &PreviewSnapshot, anchors: &[CardAnchor]) -> anyhow::Result<Vec<CardTarget>> {
    let mut targets = Vec::new();
    for (location, cards, offset, count) in [
        (CardLocation::MonsterZone, &snapshot.monsters, 0, 7),
        (CardLocation::SpellTrapZone, &snapshot.spell_traps, 7, 6),
    ] {
        for (sequence, data) in cards.iter().enumerate() {
            let address = CardAddress {
                controller: Player::Player,
                location,
                sequence: u32::try_from(sequence)?,
            };
            if sequence >= count {
                ensure!(
                    data.is_none() && snapshot.cards.card_at(address).is_none(),
                    "No visible field zone for {address:?}"
                );
                continue;
            }
            match (snapshot.cards.card_at(address), data) {
                (Some(card), Some(data)) => {
                    ensure!(
                        data.controller == address.controller
                            && data.location == address.location
                            && u32::from(data.sequence) == address.sequence
                            && (card.code == 0 || data.card_code == card.code),
                        "Tracked card and field query disagree at {address:?}"
                    );
                    targets.push(CardTarget {
                        id: card.id,
                        anchor: *anchors
                            .get(offset + sequence)
                            .context("Field anchor missing")?,
                        data: *data,
                        rotation: 0.0,
                        stack_order: 1,
                        interactive: true,
                    });
                }
                (None, None) => {}
                _ => anyhow::bail!("Tracked card and field occupancy disagree at {address:?}"),
            }
        }
        let tracked_count = snapshot
            .cards
            .cards()
            .iter()
            .filter(|card| {
                matches!(card.placement, CardPlacement::Zone(address)
                if address.controller == Player::Player && address.location == location)
            })
            .count();
        ensure!(
            tracked_count == cards.iter().flatten().count(),
            "Tracked {location} count differs from the field"
        );
    }
    Ok(targets)
}

#[component]
pub(super) fn FieldPreview(enabled: Signal<bool>) -> Element {
    let state = use_context::<UIState>();
    let tracking = use_context::<Signal<Option<CardTracking>>>();
    let anchors = use_context::<LayoutAnchors>();
    let revision = use_context::<LayoutRevision>();
    let resource = use_resource(move || {
        let enabled = enabled();
        let revision = (revision.0)();
        let anchors = anchors.clone();
        async move {
            if enabled {
                let result = measure_field(&anchors).await.map_err(|error| {
                    let error = format!("{error:#}");
                    tracing::error!("Layered field unavailable: {error}");
                    error
                });
                Some((revision, result))
            } else {
                None
            }
        }
    });
    let current = snapshot(
        enabled(),
        tracking(),
        (state.monsters)(),
        (state.spell_traps)(),
    );
    let current_revision = (revision.0)();
    let result = resource.read();
    let geometry =
        result
            .as_ref()
            .and_then(Option::as_ref)
            .and_then(|(measured_revision, result)| {
                (enabled() && (*measured_revision == current_revision || result.is_ok()))
                    .then_some(result)
            });
    let resolved = match (current.as_ref(), geometry) {
        (Some(input), Some(Ok(anchors))) => {
            Some(targets(input, anchors).map_err(|error| format!("{error:#}")))
        }
        (_, Some(Err(error))) => Some(Err(error.clone())),
        _ => None,
    };
    let layered = matches!(&resolved, Some(Ok(_)));
    let visible_targets = resolved
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let cards = current
        .as_ref()
        .map(|snapshot| snapshot.cards.cards().to_vec())
        .unwrap_or_default();
    rsx! {
        Field { layered }
        if enabled() {
            PersistentCardLayer { cards, targets: visible_targets, on_card_click: |_| {} }
        }
        if let Some(Err(error)) = resolved {
            div {
                class: "fixed top-16 left-1/2 z-50 max-w-lg -translate-x-1/2 rounded bg-red-950 p-3 text-sm text-white shadow-lg",
                role: "alert",
                "Layered field unavailable; using the legacy field: {error}"
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/field_preview.rs"]
mod tests;
