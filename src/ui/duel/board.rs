use anyhow::Context;
use dioxus::prelude::*;
use std::collections::{BTreeSet, HashMap};

use super::card_layout::{
    CardLayoutTarget, LayoutGeometry, gather_hand, preserve_pile_images, shuffle_source_cards,
    shuffle_source_targets, snapshot, targets,
};
use super::field::FieldGrid;
use super::hand::HandLayout;
use super::layout_anchors::{AnchorNotReady, LayoutAnchorId, LayoutAnchorRegistry, LayoutRevision};
use super::persistent_card_layer::{PersistentCardLayer, card_image};
use crate::duel::cards::{CardAddress, CardInstanceId, CardTracker};
use crate::duel::controller::ChainLink;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ui::card_animation::{CardAnimationState, wait_millis};
use crate::ui::components::CardImage;
use crate::utility::{card_image_url, card_thumbnail_url};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum LayoutStatus {
    #[default]
    Pending,
    Ready,
    Failed(String),
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

async fn measure_field(anchors: &LayoutAnchorRegistry) -> anyhow::Result<LayoutGeometry> {
    let mut measured = HashMap::with_capacity(17);
    for (location, count) in [
        (CardLocation::MonsterZone, 7),
        (CardLocation::SpellTrapZone, 6),
    ] {
        for sequence in 0..count {
            let address = CardAddress {
                controller: Player::Local,
                location,
                sequence,
            };
            measured.insert(
                LayoutAnchorId::Zone(location, sequence),
                anchors
                    .measure(address)
                    .await
                    .with_context(|| format!("Cannot measure {address:?}"))?,
            );
        }
    }
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let address = CardAddress {
            controller: Player::Local,
            location,
            sequence: 0,
        };
        measured.insert(
            LayoutAnchorId::Pile(location),
            anchors
                .measure(address)
                .await
                .with_context(|| format!("Cannot measure {address:?}"))?,
        );
    }
    Ok(LayoutGeometry {
        anchors: measured,
        hand_base: anchors
            .measure_hand_base()
            .await
            .context("Cannot measure hand")?,
    })
}

async fn measure_field_when_ready(
    anchors: &LayoutAnchorRegistry,
) -> anyhow::Result<LayoutGeometry> {
    for _ in 0..10 {
        match measure_field(anchors).await {
            Err(error) if error.is::<AnchorNotReady>() => wait_millis(100).await,
            result => return result,
        }
    }
    measure_field(anchors).await
}

#[component]
pub(super) fn DuelBoard(
    tracking: ReadSignal<Option<CardTracker>>,
    selected: ReadSignal<Option<CardInstanceId>>,
    animation: ReadSignal<CardAnimationState>,
    preloading: bool,
    effects_open: bool,
    deck_codes: Vec<u32>,
    onlayout: EventHandler<LayoutStatus>,
) -> Element {
    let anchors = use_context::<LayoutAnchorRegistry>();
    let revision = use_context::<LayoutRevision>();
    let mut card_layer_mounted = use_signal(|| false);
    let artwork = use_signal(HashMap::<String, ArtworkStatus>::new);
    let chain_links = use_context::<Memo<Vec<ChainLink>>>();
    let resource = use_resource(move || {
        let revision = (revision.0)();
        let anchors = anchors.clone();
        async move {
            let result = measure_field_when_ready(&anchors)
                .await
                .map_err(|error| format!("{error:#}"));
            (revision, result)
        }
    });
    let current = snapshot(tracking(), selected());
    let current_revision = (revision.0)();
    let result = resource.read();
    let geometry = result.as_ref().and_then(|(measured_revision, result)| {
        (*measured_revision == current_revision || result.is_ok()).then_some(result)
    });
    let resolved = match (current.as_ref(), geometry) {
        (Some(input), Some(Ok(geometry))) => Some(
            targets(input, geometry)
                .and_then(|mut targets| {
                    let playback = animation();
                    preserve_pile_images(input, &mut targets, &playback)?;
                    shuffle_source_targets(&mut targets, geometry.hand_base, &playback)?;
                    gather_hand(geometry.hand_base, &mut targets, &playback);
                    Ok(targets)
                })
                .map_err(|error| format!("{error:#}")),
        ),
        _ => None,
    };
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
    if preloading {
        required_artwork.extend(field_artwork);
    }
    let artwork_state = artwork();
    let status = layout_status(
        card_layer_mounted(),
        current_revision,
        result.as_ref(),
        resolved.as_ref(),
        artwork_status(&required_artwork, &artwork_state),
        preloading,
    );
    use_effect(use_reactive(&status, move |status| onlayout.call(status)));
    let targets_ready = matches!(&resolved, Some(Ok(_)));
    let visible_targets = resolved
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let mut cards = current
        .as_ref()
        .map(|snapshot| snapshot.cards.cards().to_vec())
        .unwrap_or_default();
    shuffle_source_cards(&mut cards, &animation());
    let hand_size = tracking
        .read()
        .as_ref()
        .and_then(|cards| cards.zone_data(CardLocation::Hand))
        .map_or(0, |cards| cards.len());
    rsx! {
        FieldGrid {}
        HandLayout { hand_size }
        // Remeasure after the card layer is first ready.
        if targets_ready {
            div {
                class: "hidden",
                onmounted: move |_| {
                    revision.invalidate();
                    card_layer_mounted.set(true);
                },
            }
        }
        PersistentCardLayer {
            cards,
            targets: visible_targets,
            selected: selected(),
            suppressed: effects_open,
            animating: animation.read().is_active(),
            moving: animation.read().moving.clone(),
            chain_links: chain_links(),
        }
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

fn layout_status(
    card_layer_mounted: bool,
    current_revision: u64,
    measured: Option<&(u64, Result<LayoutGeometry, String>)>,
    targets: Option<&Result<Vec<CardLayoutTarget>, String>>,
    artwork: (Option<String>, bool),
    preloading: bool,
) -> LayoutStatus {
    if let Some(Err(error)) = targets {
        return LayoutStatus::Failed(error.clone());
    }
    if preloading && let Some(url) = artwork.0 {
        return LayoutStatus::Failed(format!("Cannot load card artwork: {url}"));
    }
    let geometry = measured
        .filter(|(revision, _)| *revision == current_revision)
        .map(|(_, result)| result);
    if let Some(Err(error)) = geometry {
        let action = if preloading { "measure" } else { "remeasure" };
        return LayoutStatus::Failed(format!("Cannot {action} the field: {error}"));
    }
    if card_layer_mounted
        && geometry.is_some_and(Result::is_ok)
        && targets.is_some_and(Result::is_ok)
        && (!preloading || artwork.1)
    {
        LayoutStatus::Ready
    } else {
        LayoutStatus::Pending
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/board.rs"]
mod tests;
