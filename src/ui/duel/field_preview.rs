use anyhow::{Context, ensure};
use dioxus::prelude::*;

use super::field::Field;
use super::hand::{Hand, hand_transform};
use super::layout_anchors::{CardAnchor, LayoutAnchors, LayoutRevision};
use super::persistent_card_layer::{CardTarget, PersistentCardLayer};
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::{CardData, HandCard};
use crate::state::{SelectedCard, UIState};
use crate::ui::card_layer::{CardAddress, CardLayer, CardPlacement};
use crate::ui::card_tracking::CardTracking;

#[derive(Clone, PartialEq)]
struct PreviewSnapshot {
    cards: CardLayer,
    monsters: Vec<Option<CardData>>,
    spell_traps: Vec<Option<CardData>>,
    hand: Vec<HandCard>,
    selected: Option<SelectedCard>,
}

fn snapshot(
    enabled: bool,
    tracking: Option<CardTracking>,
    monsters: Vec<Option<CardData>>,
    spell_traps: Vec<Option<CardData>>,
    hand: Vec<HandCard>,
    selected: Option<SelectedCard>,
) -> Option<PreviewSnapshot> {
    match (enabled, tracking) {
        (true, Some(CardTracking::Active(cards))) => Some(PreviewSnapshot {
            cards,
            monsters,
            spell_traps,
            hand,
            selected,
        }),
        _ => None,
    }
}

struct PreviewGeometry {
    field: Vec<CardAnchor>,
    hand_base: CardAnchor,
}

fn hand_position(
    base: CardAnchor,
    index: usize,
    count: usize,
    selected: bool,
) -> (CardAnchor, f64, usize) {
    let (distance, default_y) = hand_transform(index, count);
    let rotation = if selected { 0 } else { distance };
    let translate_y = if selected { -25 } else { default_y };
    let angle = f64::from(rotation).to_radians();
    let shift = base.height * f64::from(translate_y) / 100.0;
    // The hand slots are 10vw wide with -1.1vw margins on both sides.
    let spacing = base.width * 0.78;
    (
        CardAnchor {
            center_x: base.center_x + (index as f64 - (count as f64 - 1.0) / 2.0) * spacing
                - angle.sin() * shift,
            center_y: base.center_y + angle.cos() * shift,
            ..base
        },
        f64::from(rotation),
        if selected { count } else { index },
    )
}

async fn measure_field(anchors: &LayoutAnchors) -> anyhow::Result<PreviewGeometry> {
    let mut field = Vec::with_capacity(13);
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
            field.push(
                anchors
                    .measure(address)
                    .await
                    .with_context(|| format!("Cannot measure {address:?}"))?,
            );
        }
    }
    Ok(PreviewGeometry {
        field,
        hand_base: anchors
            .measure_hand_base()
            .await
            .context("Cannot measure hand")?,
    })
}

fn targets(
    snapshot: &PreviewSnapshot,
    geometry: &PreviewGeometry,
) -> anyhow::Result<Vec<CardTarget>> {
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
                        anchor: *geometry
                            .field
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
    let count = snapshot.hand.len();
    let tracked_count = snapshot
        .cards
        .cards()
        .iter()
        .filter(|card| {
            matches!(card.placement, CardPlacement::Zone(address)
            if address.controller == Player::Player && address.location == CardLocation::Hand)
        })
        .count();
    ensure!(
        tracked_count == count,
        "Tracked hand count differs from the hand query"
    );
    for (index, card) in snapshot.hand.iter().enumerate() {
        ensure!(card.index == index, "Hand query sequence is out of order");
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: u32::try_from(index)?,
        };
        let tracked = snapshot
            .cards
            .card_at(address)
            .with_context(|| format!("Untracked hand card at {address:?}"))?;
        ensure!(
            tracked.code == 0 || tracked.code == card.code,
            "Tracked hand card differs at {address:?}"
        );
        let selected = snapshot
            .selected
            .is_some_and(|selected| selected.matches(address));
        let (anchor, rotation, stack) = hand_position(geometry.hand_base, index, count, selected);
        targets.push(CardTarget {
            id: tracked.id,
            anchor,
            data: CardData {
                card_code: card.code,
                controller: Player::Player,
                location: CardLocation::Hand,
                sequence: u8::try_from(index)?,
                ..Default::default()
            },
            rotation,
            stack_order: 50 + i32::try_from(stack)?,
            interactive: true,
        });
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
        (state.hand_contents)(),
        (state.selected_card)(),
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
        (Some(input), Some(Ok(geometry))) => {
            Some(targets(input, geometry).map_err(|error| format!("{error:#}")))
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
        Hand { layered }
        if enabled() {
            PersistentCardLayer { cards, targets: visible_targets }
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
