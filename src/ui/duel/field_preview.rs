use anyhow::{Context, ensure};
use dioxus::prelude::*;

use super::field::Field;
use super::hand::{Hand, hand_transform};
use super::layout_anchors::{CardAnchor, LayoutAnchors, LayoutRevision};
use super::persistent_card_layer::{CardAppearance, CardTarget, PersistentCardLayer};
use crate::ocgcore::CardType;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Player};
use crate::ocgcore::{CardData, HandCard};
use crate::state::{SelectedCard, UIState};
use crate::ui::card_layer::{CardAddress, CardLayer, CardPlacement};
use crate::ui::card_tracking::CardTracking;
use crate::ui::components::CardImage;

#[derive(Clone, PartialEq)]
struct PreviewSnapshot {
    cards: CardLayer,
    monsters: Vec<Option<CardData>>,
    spell_traps: Vec<Option<CardData>>,
    hand: Vec<HandCard>,
    selected: Option<SelectedCard>,
    deck_count: usize,
    extra: Vec<Option<CardData>>,
    graveyard: Vec<Option<CardData>>,
    banishment: Vec<Option<CardData>>,
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
            deck_count: 0,
            extra: Vec::new(),
            graveyard: Vec::new(),
            banishment: Vec::new(),
        }),
        _ => None,
    }
}

struct PreviewGeometry {
    field: Vec<CardAnchor>,
    hand_base: CardAnchor,
    piles: Vec<CardAnchor>,
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
    let mut piles = Vec::new();
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let address = CardAddress {
            controller: Player::Player,
            location,
            sequence: 0,
        };
        piles.push(
            anchors
                .measure(address)
                .await
                .with_context(|| format!("Cannot measure {address:?}"))?,
        );
    }
    Ok(PreviewGeometry {
        field,
        piles,
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
                        address,
                        anchor: *geometry
                            .field
                            .get(offset + sequence)
                            .context("Field anchor missing")?,
                        data: *data,
                        rotation: 0.0,
                        stack_order: 2,
                        interactive: true,
                        appearance: CardAppearance::Interactive,
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
            address,
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
            appearance: CardAppearance::Interactive,
        });
    }
    for (pile_index, (location, count, queried)) in [
        (CardLocation::Deck, snapshot.deck_count, None),
        (
            CardLocation::ExtraDeck,
            snapshot.extra.len(),
            Some(&snapshot.extra),
        ),
        (
            CardLocation::Graveyard,
            snapshot.graveyard.len(),
            Some(&snapshot.graveyard),
        ),
        (
            CardLocation::Banishment,
            snapshot.banishment.len(),
            Some(&snapshot.banishment),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let tracked_count = snapshot.cards.cards().iter().filter(|card| matches!(card.placement,
            CardPlacement::Zone(address) if address.controller == Player::Player && address.location == location)).count();
        ensure!(
            tracked_count == count,
            "Tracked {location} count differs from the pile"
        );
        let image = match location {
            CardLocation::Deck => CardImage::CardBack,
            CardLocation::ExtraDeck => CardImage::ExtraDeckBack,
            CardLocation::Graveyard | CardLocation::Banishment if count > 0 => {
                let top = queried
                    .context("Pile query missing")?
                    .last()
                    .and_then(|card| *card)
                    .context("Pile top missing")?;
                if location == CardLocation::Banishment
                    && !matches!(
                        top.position,
                        Some(BattlePosition::FaceUpAttack | BattlePosition::FaceUpDefense)
                    )
                {
                    if top.card_type.intersects(
                        CardType::FUSION | CardType::SYNCHRO | CardType::XYZ | CardType::LINK,
                    ) {
                        CardImage::ExtraDeckBack
                    } else {
                        CardImage::CardBack
                    }
                } else if top.card_code == 0 {
                    CardImage::CardBack
                } else {
                    CardImage::Thumbnail(top)
                }
            }
            _ => continue,
        };
        for sequence in 0..count {
            let address = CardAddress {
                controller: Player::Player,
                location,
                sequence: u32::try_from(sequence)?,
            };
            let card = snapshot
                .cards
                .card_at(address)
                .with_context(|| format!("Untracked pile card at {address:?}"))?;
            if let Some(queried) = queried {
                let data = queried[sequence].context("Empty pile query slot")?;
                ensure!(
                    data.controller == address.controller
                        && data.location == location
                        && u32::from(data.sequence) == address.sequence
                        && (card.code == 0 || data.card_code == 0 || card.code == data.card_code),
                    "Tracked card and pile query disagree at {address:?}"
                );
            }
            targets.push(CardTarget {
                id: card.id,
                address,
                anchor: *geometry
                    .piles
                    .get(pile_index)
                    .context("Pile anchor missing")?,
                data: CardData {
                    card_code: card.code,
                    controller: Player::Player,
                    location,
                    sequence: u8::try_from(sequence)?,
                    ..Default::default()
                },
                rotation: if location == CardLocation::Banishment {
                    -90.0
                } else {
                    0.0
                },
                stack_order: 1,
                interactive: false,
                appearance: CardAppearance::Pile(image),
            });
        }
    }
    for card in snapshot.cards.cards() {
        let CardPlacement::Overlay { host, sequence } = card.placement else {
            continue;
        };
        let host = snapshot
            .cards
            .card_by_id(host)
            .context("Material host missing")?;
        let CardPlacement::Zone(address) = host.placement else {
            anyhow::bail!("Material host is not in a zone")
        };
        if address.controller != Player::Player {
            continue;
        }
        let host_target = targets
            .iter()
            .find(|target| target.id == host.id)
            .context("Material host has no visible target")?;
        let mut anchor = host_target.anchor;
        anchor.center_x += anchor.width * (0.12 + f64::from(sequence) * 0.06);
        anchor.center_y += anchor.height * 0.12;
        targets.push(CardTarget {
            id: card.id,
            address,
            anchor,
            data: CardData {
                card_code: card.code,
                controller: address.controller,
                location: address.location,
                sequence: u8::try_from(address.sequence)?,
                ..Default::default()
            },
            rotation: 0.0,
            stack_order: 1,
            interactive: false,
            appearance: CardAppearance::Material,
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
    let mut current = snapshot(
        enabled(),
        tracking(),
        (state.monsters)(),
        (state.spell_traps)(),
        (state.hand_contents)(),
        (state.selected_card)(),
    );
    if let Some(current) = &mut current {
        current.deck_count = (state.main_deck_length)() as usize;
        current.extra = (state.extra_deck)();
        current.graveyard = (state.graveyard)();
        current.banishment = (state.banishment)();
    }
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
