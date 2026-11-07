use anyhow::{Context, ensure};
use dioxus::prelude::*;
use std::collections::{BTreeSet, HashMap};

use super::field::Field;
use super::hand::{Hand, hand_transform};
use super::layout_anchors::{AnchorNotReady, CardAnchor, LayoutAnchors, LayoutRevision};
use super::persistent_card_layer::{CardAppearance, CardTarget, PersistentCardLayer, card_image};
use crate::ocgcore::CardType;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Player};
use crate::ocgcore::{CardData, HandCard};
use crate::state::{SelectedCard, UIState};
use crate::ui::card_layer::{CardAddress, CardLayer, CardPlacement};
use crate::ui::card_playback::{CardPlayback, wait_millis};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArtworkStatus {
    Loaded,
    Failed,
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

async fn measure_field_when_ready(anchors: &LayoutAnchors) -> anyhow::Result<PreviewGeometry> {
    for _ in 0..10 {
        match measure_field(anchors).await {
            Err(error) if error.is::<AnchorNotReady>() => wait_millis(100).await,
            result => return result,
        }
    }
    measure_field(anchors).await
}

fn preserve_pile_images(
    snapshot: &PreviewSnapshot,
    targets: &mut [CardTarget],
    playback: &CardPlayback,
) -> anyhow::Result<()> {
    let Some(source) = playback.source.as_ref().filter(|_| playback.active) else {
        return Ok(());
    };
    for (location, index) in [(CardLocation::Graveyard, 1), (CardLocation::Banishment, 2)] {
        let surviving_top = source
            .cards()
            .iter()
            .filter_map(|card| {
                let CardPlacement::Zone(old) = card.placement else {
                    return None;
                };
                let current = snapshot.cards.card_by_id(card.id)?;
                let CardPlacement::Zone(new) = current.placement else {
                    return None;
                };
                (old.controller == Player::Player
                    && old.location == location
                    && new.controller == old.controller
                    && new.location == location)
                    .then_some(old.sequence)
            })
            .max();
        let old_image = if let Some(sequence) = surviving_top {
            let data = playback.piles[index]
                .get(usize::try_from(sequence)?)
                .copied()
                .flatten()
                .context("Previous pile card is missing")?;
            Some(pile_face(location, data))
        } else {
            None
        };
        for target in targets.iter_mut().filter(|target| {
            target.address.location == location
                && matches!(target.appearance, CardAppearance::Pile(_))
        }) {
            let was_in_pile = source.card_by_id(target.id).is_some_and(|card| {
                matches!(
                    card.placement, CardPlacement::Zone(address)
                        if address.controller == Player::Player && address.location == location
                )
            });
            if was_in_pile {
                target.appearance =
                    CardAppearance::Pile(old_image.context("Previous pile image is missing")?);
            } else if matches!(
                target.appearance,
                CardAppearance::Pile(CardImage::Thumbnail(_))
            ) {
                target.appearance = CardAppearance::Pile(if target.data.card_code == 0 {
                    CardImage::CardBack
                } else {
                    CardImage::Thumbnail(target.data)
                });
            }
        }
    }
    Ok(())
}

fn pile_face(location: CardLocation, top: CardData) -> CardImage {
    if location == CardLocation::Banishment
        && !matches!(
            top.position,
            Some(BattlePosition::FaceUpAttack | BattlePosition::FaceUpDefense)
        )
    {
        if top
            .card_type
            .intersects(CardType::FUSION | CardType::SYNCHRO | CardType::XYZ | CardType::LINK)
        {
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
                pile_face(location, top)
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
pub(super) fn FieldPreview(enabled: Signal<bool>, mut startup_ready: Signal<bool>) -> Element {
    let state = use_context::<UIState>();
    let tracking = use_context::<Signal<Option<CardTracking>>>();
    let anchors = use_context::<LayoutAnchors>();
    let revision = use_context::<LayoutRevision>();
    let mut layered_mounted = use_signal(|| false);
    let mut artwork = use_signal(HashMap::<String, ArtworkStatus>::new);
    let playback = try_consume_context::<Signal<CardPlayback>>();
    let resource = use_resource(move || {
        let enabled = enabled();
        let layout_revision = revision;
        let revision = (revision.0)();
        let anchors = anchors.clone();
        async move {
            if enabled {
                let result = measure_field_when_ready(&anchors).await.map_err(|error| {
                    let error = format!("{error:#}");
                    if *layout_revision.0.peek() == revision {
                        tracing::error!("Layered field unavailable: {error}");
                    }
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
        (Some(input), Some(Ok(geometry))) => Some(
            targets(input, geometry)
                .and_then(|mut targets| {
                    if let Some(playback) = playback {
                        preserve_pile_images(input, &mut targets, &playback())?;
                    }
                    Ok(targets)
                })
                .map_err(|error| format!("{error:#}")),
        ),
        (_, Some(Err(error))) => Some(Err(error.clone())),
        _ => None,
    };
    let target_failure = if matches!(geometry, Some(Ok(_))) {
        resolved
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .cloned()
    } else {
        None
    };
    let targets_ready = resolved.as_ref().is_some_and(Result::is_ok);
    let required_artwork: BTreeSet<String> = current
        .as_ref()
        .zip(resolved.as_ref().and_then(|result| result.as_ref().ok()))
        .map(|(snapshot, targets)| {
            snapshot
                .cards
                .cards()
                .iter()
                .filter_map(|card| {
                    card_image(
                        *card,
                        targets.iter().find(|target| target.id == card.id).copied(),
                    )
                })
                .map(CardImage::url)
                .collect()
        })
        .unwrap_or_default();
    let artwork_state = artwork();
    let artwork_failure = required_artwork
        .iter()
        .find(|url| artwork_state.get(*url) == Some(&ArtworkStatus::Failed))
        .cloned();
    let artwork_ready = required_artwork
        .iter()
        .all(|url| artwork_state.get(url) == Some(&ArtworkStatus::Loaded));
    use_effect(move || {
        let current_revision = (revision.0)();
        let measured = resource.read();
        let mounted = layered_mounted();
        let ready = startup_ready();
        let waiting = (state.waiting_on_input)();
        let _ = artwork();
        let _ = enabled();
        let _ = tracking();
        let _ = (state.monsters)();
        let _ = (state.spell_traps)();
        let _ = (state.hand_contents)();
        let _ = (state.main_deck_length)();
        let _ = (state.extra_deck)();
        let _ = (state.graveyard)();
        let _ = (state.banishment)();
        let Some(mut playback) = playback else { return };
        let current = playback();
        if !current.enabled || current.failure.is_some() || current.restarting {
            return;
        }
        if let Some(error) = &target_failure {
            tracing::error!("Card layer synchronization failed: {error}");
            playback.write().fail(error.clone());
            return;
        }
        if !ready {
            if let Some(url) = &artwork_failure {
                let error = format!("Cannot load card artwork: {url}");
                tracing::error!("{error}");
                playback.write().fail(error);
                return;
            }
            if let Some((measured_revision, result)) = measured.as_ref().and_then(Option::as_ref) {
                if *measured_revision == current_revision {
                    match result {
                        Err(error) => {
                            playback
                                .write()
                                .fail(format!("Cannot measure the field: {error}"));
                            return;
                        }
                        Ok(_)
                            if startup_layout_ready(
                                mounted,
                                *measured_revision,
                                current_revision,
                                targets_ready,
                                artwork_ready,
                                waiting || current.finished,
                                current.blocked(),
                            ) =>
                        {
                            startup_ready.set(true)
                        }
                        Ok(_) => {}
                    }
                }
            }
        }
        if !current.layout_pending {
            return;
        }
        if let Some((measured_revision, result)) = measured.as_ref().and_then(Option::as_ref) {
            if *measured_revision == current_revision {
                match result {
                    Ok(_) => playback.write().layout_pending = false,
                    Err(error) => playback
                        .write()
                        .fail(format!("Cannot remeasure the field: {error}")),
                }
            }
        }
    });
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
        // The hand layout can shift when legacy cards are replaced.
        if layered {
            div {
                class: "hidden",
                onmounted: move |_| {
                    revision.invalidate();
                    layered_mounted.set(true);
                },
            }
        }
        if enabled() {
            PersistentCardLayer { cards, targets: visible_targets }
        }
        if !startup_ready() {
            for url in required_artwork {
                {
                    let loaded_url = url.clone();
                    let failed_url = url.clone();
                    rsx! {
                        img {
                            key: "{url}",
                            class: "hidden",
                            alt: "",
                            src: "{url}",
                            onload: move |_| {
                                artwork.write().insert(loaded_url.clone(), ArtworkStatus::Loaded);
                            },
                            onerror: move |_| {
                                artwork.write().insert(failed_url.clone(), ArtworkStatus::Failed);
                            },
                        }
                    }
                }
            }
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

fn startup_layout_ready(
    layered_mounted: bool,
    measured_revision: u64,
    current_revision: u64,
    targets_ready: bool,
    artwork_ready: bool,
    duel_settled: bool,
    playback_blocked: bool,
) -> bool {
    layered_mounted
        && measured_revision == current_revision
        && targets_ready
        && artwork_ready
        && duel_settled
        && !playback_blocked
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/field_preview.rs"]
mod tests;
