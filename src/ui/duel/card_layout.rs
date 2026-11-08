use anyhow::{Context, ensure};
use std::collections::HashMap;

use super::layout_anchors::{Anchor, CardAnchor};
use crate::duel::cards::{Card, CardAddress, CardId, CardPlacement, CardTracker};
use crate::ocgcore::CardData;
use crate::ocgcore::CardType;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Player};
use crate::ui::card_animation::{AnimationPhase, CardAnimationState};
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
    pub appearance: CardAppearance,
}

#[derive(Clone, PartialEq)]
pub(super) struct PreviewSnapshot {
    pub cards: CardTracker,
    pub selected: Option<CardId>,
}

pub(super) fn snapshot(
    tracking: Option<CardTracker>,
    selected: Option<CardId>,
) -> Option<PreviewSnapshot> {
    match tracking {
        Some(cards)
            if cards.cards().iter().all(|card| {
                !matches!(card.placement, CardPlacement::Zone(address) if address.controller == Player::Player)
                    || card.data.is_some()
            }) => Some(PreviewSnapshot { cards, selected }),
        _ => None,
    }
}

pub(super) struct PreviewGeometry {
    pub anchors: HashMap<Anchor, CardAnchor>,
    pub hand_base: CardAnchor,
}

fn hand_transform(index: usize, hand_size: usize) -> (i32, i32) {
    let center = hand_size as i32 - 1;
    let distance = index as i32 * 2 - center;
    let translate_y = (distance.abs() * distance.abs() * 4) / 16;
    (distance, translate_y)
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

pub(super) fn shuffle_source_cards(cards: &mut Vec<Card>, playback: &CardAnimationState) {
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

pub(super) fn shuffle_source_targets(
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

pub(super) fn gather_hand(
    base: CardAnchor,
    targets: &mut [CardTarget],
    playback: &CardAnimationState,
) {
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

pub(super) fn preserve_pile_images(
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

pub(super) fn targets(
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
                    let count = if location == CardLocation::MonsterZone {
                        7
                    } else {
                        6
                    };
                    ensure!(
                        address.sequence < count,
                        "No visible field zone for {address:?}"
                    );
                    (
                        *geometry
                            .anchors
                            .get(&Anchor::Zone(location, address.sequence))
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
                _ => (
                    *geometry
                        .anchors
                        .get(&Anchor::Pile(location))
                        .context("Pile anchor missing")?,
                    if location == CardLocation::Banishment {
                        -90.0
                    } else {
                        0.0
                    },
                    1,
                    CardAppearance::Pile(pile_image.context("Pile image is missing")?),
                ),
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

#[cfg(test)]
#[path = "../../../tests/unit/ui/card_layout.rs"]
mod tests;
