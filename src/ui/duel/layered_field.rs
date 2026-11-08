use anyhow::{Context, ensure};
use dioxus::prelude::*;
use std::collections::{BTreeSet, HashMap};

use super::field::Field;
use super::hand::{Hand, hand_transform};
use super::layout_anchors::{AnchorNotReady, CardAnchor, LayoutAnchors, LayoutRevision};
use super::persistent_card_layer::{CardAppearance, CardTarget, PersistentCardLayer, card_image};
use crate::duel::cards::{Card, CardAddress, CardId, CardPlacement, CardTracker};
use crate::ocgcore::CardData;
use crate::ocgcore::CardType;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Player};
use crate::state::UIState;
use crate::ui::card_animation::{AnimationPhase, CardAnimationState, wait_millis};
use crate::ui::components::CardImage;
use crate::ui::playback::Playback;
use crate::ui::view_state::ViewState;
use crate::utility::{card_image_url, card_thumbnail_url};

#[derive(Clone, PartialEq)]
struct PreviewSnapshot {
    cards: CardTracker,
    selected: Option<CardId>,
}

fn snapshot(tracking: Option<CardTracker>, selected: Option<CardId>) -> Option<PreviewSnapshot> {
    match tracking {
        Some(cards)
            if cards.cards().iter().all(|card| {
                !matches!(card.placement, CardPlacement::Zone(address) if address.controller == Player::Player)
                    || card.data.is_some()
            }) => Some(PreviewSnapshot { cards, selected }),
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

fn deck_artwork_urls(codes: &[u32]) -> BTreeSet<String> {
    let mut urls = BTreeSet::from([CardImage::CardBack.url(), CardImage::ExtraDeckBack.url()]);
    for &code in codes {
        urls.insert(card_image_url(code));
        urls.insert(card_thumbnail_url(code));
    }
    urls
}

fn artwork_status(
    urls: &BTreeSet<String>,
    artwork: &HashMap<String, ArtworkStatus>,
) -> (Option<String>, bool) {
    let failed = urls
        .iter()
        .find(|url| artwork.get(*url) == Some(&ArtworkStatus::Failed))
        .cloned();
    let ready = urls
        .iter()
        .all(|url| artwork.get(url) == Some(&ArtworkStatus::Loaded));
    (failed, ready)
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

fn local_hand(card: &Card) -> bool {
    matches!(card.placement, CardPlacement::Zone(address)
        if address.controller == Player::Player && address.location == CardLocation::Hand)
}

fn shuffle_source_cards(cards: &mut Vec<Card>, playback: &CardAnimationState) {
    if let Some(source) = playback
        .source
        .as_ref()
        .filter(|_| playback.phase == AnimationPhase::HandShuffleGathering)
    {
        cards.retain(|card| !local_hand(card));
        cards.extend(
            source
                .cards()
                .iter()
                .filter(|card| local_hand(card))
                .copied(),
        );
    }
}

fn shuffle_source_targets(
    targets: &mut Vec<CardTarget>,
    base: CardAnchor,
    playback: &CardAnimationState,
) -> anyhow::Result<()> {
    if let Some(source) = playback
        .source
        .as_ref()
        .filter(|_| playback.phase == AnimationPhase::HandShuffleGathering)
    {
        targets.retain(|target| target.address.location != CardLocation::Hand);
        for card in source.cards().iter().filter(|card| local_hand(card)) {
            let CardPlacement::Zone(address) = card.placement else {
                unreachable!()
            };
            targets.push(CardTarget {
                id: card.id,
                address,
                anchor: base,
                data: CardData {
                    card_code: card.data.map_or(card.code, |data| data.card_code),
                    controller: address.controller,
                    location: address.location,
                    sequence: u8::try_from(address.sequence)?,
                    ..Default::default()
                },
                rotation: 0.0,
                stack_order: 50 + i32::try_from(address.sequence)?,
                appearance: CardAppearance::Interactive,
            });
        }
    }
    Ok(())
}

fn gather_hand(base: CardAnchor, targets: &mut [CardTarget], playback: &CardAnimationState) {
    if matches!(
        playback.phase,
        AnimationPhase::HandShuffleGathering | AnimationPhase::HandShuffleStacking
    ) {
        for target in targets.iter_mut().filter(|target| {
            target.address.controller == Player::Player
                && target.address.location == CardLocation::Hand
        }) {
            target.anchor = base;
            target.rotation = 0.0;
        }
    }
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
    playback: &CardAnimationState,
) -> anyhow::Result<()> {
    let Some(source) = playback.source.as_ref().filter(|_| playback.is_active()) else {
        return Ok(());
    };
    for location in [CardLocation::Graveyard, CardLocation::Banishment] {
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
            let data = source
                .card_at(CardAddress {
                    controller: Player::Player,
                    location,
                    sequence,
                })
                .and_then(|card| card.data)
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
    for location in [
        CardLocation::MonsterZone,
        CardLocation::SpellTrapZone,
        CardLocation::Hand,
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let mut cards: Vec<_> = snapshot
            .cards
            .cards()
            .iter()
            .filter_map(|card| match card.placement {
                CardPlacement::Zone(address)
                    if address.controller == Player::Player && address.location == location =>
                {
                    Some((address, card))
                }
                _ => None,
            })
            .collect();
        cards.sort_by_key(|(address, _)| address.sequence);
        let count = cards.len();
        let pile_image = match location {
            CardLocation::Deck => Some(CardImage::CardBack),
            CardLocation::ExtraDeck => Some(CardImage::ExtraDeckBack),
            CardLocation::Graveyard | CardLocation::Banishment => cards
                .last()
                .map(|(_, card)| {
                    card.data
                        .map(|data| pile_face(location, data))
                        .context("Pile top data is missing")
                })
                .transpose()?,
            _ => None,
        };
        for (index, (address, card)) in cards.into_iter().enumerate() {
            let data = card
                .data
                .with_context(|| format!("Card data is missing at {address:?}"))?;
            let (anchor, rotation, stack_order, appearance) = match location {
                CardLocation::MonsterZone | CardLocation::SpellTrapZone => {
                    let sequence = usize::try_from(address.sequence)?;
                    let (offset, count) = if location == CardLocation::MonsterZone {
                        (0, 7)
                    } else {
                        (7, 6)
                    };
                    ensure!(sequence < count, "No visible field zone for {address:?}");
                    (
                        *geometry
                            .field
                            .get(offset + sequence)
                            .context("Field anchor missing")?,
                        0.0,
                        2,
                        CardAppearance::Interactive,
                    )
                }
                CardLocation::Hand => {
                    let (anchor, rotation, stack) = hand_position(
                        geometry.hand_base,
                        index,
                        count,
                        snapshot.selected == Some(card.id),
                    );
                    (
                        anchor,
                        rotation,
                        50 + i32::try_from(stack)?,
                        CardAppearance::Interactive,
                    )
                }
                _ => {
                    let pile_index = match location {
                        CardLocation::Deck => 0,
                        CardLocation::ExtraDeck => 1,
                        CardLocation::Graveyard => 2,
                        CardLocation::Banishment => 3,
                        _ => unreachable!(),
                    };
                    (
                        *geometry
                            .piles
                            .get(pile_index)
                            .context("Pile anchor missing")?,
                        if location == CardLocation::Banishment {
                            -90.0
                        } else {
                            0.0
                        },
                        1,
                        CardAppearance::Pile(pile_image.context("Pile image is missing")?),
                    )
                }
            };
            let interactive = appearance == CardAppearance::Interactive;
            targets.push(CardTarget {
                id: card.id,
                address,
                anchor,
                data: if matches!(
                    location,
                    CardLocation::MonsterZone | CardLocation::SpellTrapZone
                ) {
                    data
                } else {
                    CardData {
                        card_code: if interactive {
                            data.card_code
                        } else {
                            card.code
                        },
                        controller: address.controller,
                        location,
                        sequence: u8::try_from(address.sequence)?,
                        ..Default::default()
                    }
                },
                rotation,
                stack_order,
                appearance,
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
            appearance: CardAppearance::Material,
        });
    }
    Ok(targets)
}

#[component]
pub(super) fn LayeredField(mut startup_ready: Signal<bool>, deck_codes: Vec<u32>) -> Element {
    let mut state = use_context::<UIState>();
    let view = use_context::<ViewState>();
    let tracking = state.cards;
    let anchors = use_context::<LayoutAnchors>();
    let revision = use_context::<LayoutRevision>();
    let mut layered_mounted = use_signal(|| false);
    let artwork = use_signal(HashMap::<String, ArtworkStatus>::new);
    let playback_state = use_context::<Playback>();
    let playback = playback_state.animation;
    let resource = use_resource(move || {
        let layout_revision = revision;
        let revision = (revision.0)();
        let anchors = anchors.clone();
        async move {
            let result = measure_field_when_ready(&anchors).await.map_err(|error| {
                let error = format!("{error:#}");
                if *layout_revision.0.peek() == revision {
                    tracing::error!("Cannot measure card layer: {error}");
                }
                error
            });
            (revision, result)
        }
    });
    let current = snapshot(tracking(), (view.selected_card)());
    let current_revision = (revision.0)();
    let result = resource.read();
    let geometry = result.as_ref().and_then(|(measured_revision, result)| {
        (*measured_revision == current_revision || result.is_ok()).then_some(result)
    });
    let resolved = match (current.as_ref(), geometry) {
        (Some(input), Some(Ok(geometry))) => Some(
            targets(input, geometry)
                .and_then(|mut targets| {
                    let playback = playback();
                    preserve_pile_images(input, &mut targets, &playback)?;
                    shuffle_source_targets(&mut targets, geometry.hand_base, &playback)?;
                    gather_hand(geometry.hand_base, &mut targets, &playback);
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
    let mut required_artwork = deck_artwork_urls(&deck_codes);
    let field_artwork: BTreeSet<String> = current
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
    if !startup_ready() {
        required_artwork.extend(field_artwork);
    }
    let artwork_state = artwork();
    let (artwork_failure, artwork_ready) = artwork_status(&required_artwork, &artwork_state);
    use_effect(move || {
        let current_revision = (revision.0)();
        let measured = resource.read();
        let mounted = layered_mounted();
        let ready = startup_ready();
        let waiting = state.waiting_on_input();
        let finished = (state.duel_finished)();
        let _ = artwork();
        let _ = tracking();
        let playing = playback_state.is_active();
        if (state.failure)().is_some() || (state.restarting)() {
            return;
        }
        if let Some(error) = &target_failure {
            tracing::error!("Card layer synchronization failed: {error}");
            state.fail(error.clone(), playback_state);
            return;
        }
        if !ready {
            if let Some(url) = &artwork_failure {
                let error = format!("Cannot load card artwork: {url}");
                tracing::error!("{error}");
                state.fail(error, playback_state);
                return;
            }
            if let Some((measured_revision, result)) = measured.as_ref() {
                if *measured_revision == current_revision {
                    match result {
                        Err(error) => {
                            state
                                .fail(format!("Cannot measure the field: {error}"), playback_state);
                            return;
                        }
                        Ok(_)
                            if startup_layout_ready(
                                mounted,
                                *measured_revision,
                                current_revision,
                                targets_ready,
                                artwork_ready,
                                waiting || finished,
                                playing,
                            ) =>
                        {
                            startup_ready.set(true)
                        }
                        Ok(_) => {}
                    }
                }
            }
        }
        if let Some((measured_revision, result)) = measured.as_ref() {
            if *measured_revision == current_revision {
                match result {
                    Ok(_) => {}
                    Err(error) => state.fail(
                        format!("Cannot remeasure the field: {error}"),
                        playback_state,
                    ),
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
    let mut cards = current
        .as_ref()
        .map(|snapshot| snapshot.cards.cards().to_vec())
        .unwrap_or_default();
    shuffle_source_cards(&mut cards, &playback());
    rsx! {
        Field {}
        Hand {}
        // Remeasure after the card layer is first ready.
        if layered {
            div {
                class: "hidden",
                onmounted: move |_| {
                    revision.invalidate();
                    layered_mounted.set(true);
                },
            }
        }
        PersistentCardLayer { cards, targets: visible_targets }
        ArtworkPreloader { urls: required_artwork, artwork }
    }
}

#[component]
fn ArtworkPreloader(
    urls: BTreeSet<String>,
    mut artwork: Signal<HashMap<String, ArtworkStatus>>,
) -> Element {
    rsx! {
        for url in urls {
            {
                let loaded_url = url.clone();
                let failed_url = url.clone();
                rsx! {
                    img {
                        key: "{url}",
                        class: "hidden",
                        alt: "",
                        loading: "eager",
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
#[path = "../../../tests/unit/ui/layered_field.rs"]
mod tests;
