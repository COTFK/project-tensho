use super::super::layout_anchors::{Anchor, AnchorPart};
use super::*;
use dioxus::html::geometry::{PixelsRect, PixelsSize, euclid::point2};
use dioxus::html::{MountedResult, RenderedElementBacking};
use std::any::Any;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

struct TestElement(PixelsRect);

impl RenderedElementBacking for TestElement {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        let rect = self.0;
        Box::pin(async move { Ok(rect) })
    }
}

fn fixture() -> (PreviewSnapshot, LayoutAnchors) {
    let mut snapshot = PreviewSnapshot {
        cards: CardLayer::default(),
        monsters: vec![None; 7],
        spell_traps: vec![None; 8],
        hand: vec![],
        selected: None,
        deck_count: 0,
        extra: vec![],
        graveyard: vec![],
        banishment: vec![],
    };
    let anchors = LayoutAnchors::default();
    anchors.insert(
        (Anchor::HandBase, AnchorPart::Card),
        Rc::new(()),
        Rc::new(MountedData::new(TestElement(PixelsRect::new(
            point2(500.0, 400.0),
            PixelsSize::new(100.0, 146.0),
        )))),
    );
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        anchors.insert(
            (Anchor::Pile(location), AnchorPart::Card),
            Rc::new(()),
            Rc::new(MountedData::new(TestElement(PixelsRect::new(
                point2(600.0, 400.0),
                PixelsSize::new(59.0, 86.0),
            )))),
        );
    }
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
            let code = 100 + sequence;
            snapshot.cards.insert(address, code).unwrap();
            let data = CardData {
                card_code: code,
                controller: Player::Player,
                location,
                sequence: u8::try_from(sequence).unwrap(),
                ..Default::default()
            };
            match location {
                CardLocation::MonsterZone => snapshot.monsters[sequence as usize] = Some(data),
                CardLocation::SpellTrapZone => snapshot.spell_traps[sequence as usize] = Some(data),
                _ => unreachable!(),
            }
            let rect = PixelsRect::new(
                point2(f64::from(sequence) * 100.0, 20.0),
                PixelsSize::new(59.0, 86.0),
            );
            anchors.insert(
                (Anchor::Zone(location, sequence), AnchorPart::Card),
                Rc::new(()),
                Rc::new(MountedData::new(TestElement(rect))),
            );
        }
    }
    (snapshot, anchors)
}

#[test]
fn measures_all_normal_extra_and_spell_trap_zones() {
    let (snapshot, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    let targets = targets(&snapshot, &geometry).unwrap();
    assert_eq!(targets.len(), 13);
    for target in targets {
        assert_eq!(
            target.anchor.center_x,
            f64::from(target.data.sequence) * 100.0 + 29.5
        );
        assert_eq!(target.anchor.center_y, 63.0);
        assert_eq!(target.rotation, 0.0);
        assert!(target.interactive);
    }
}

#[test]
fn retries_an_anchor_that_is_initially_zero_sized() {
    struct DelayedElement(Cell<u32>);

    impl RenderedElementBacking for DelayedElement {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
            let attempts = self.0.get();
            self.0.set(attempts + 1);
            let size = if attempts == 0 {
                PixelsSize::new(0.0, 0.0)
            } else {
                PixelsSize::new(59.0, 86.0)
            };
            Box::pin(async move { Ok(PixelsRect::new(point2(0.0, 0.0), size)) })
        }
    }

    let (_, anchors) = fixture();
    anchors.insert(
        (Anchor::Zone(CardLocation::MonsterZone, 0), AnchorPart::Card),
        Rc::new(()),
        Rc::new(MountedData::new(DelayedElement(Cell::new(0)))),
    );
    assert!(futures::executor::block_on(measure_field_when_ready(&anchors)).is_ok());
}

#[test]
fn persistent_missing_anchors_are_reported() {
    let error =
        match futures::executor::block_on(measure_field_when_ready(&LayoutAnchors::default())) {
            Ok(_) => panic!("missing anchors should fail"),
            Err(error) => error,
        };
    assert!(error.is::<AnchorNotReady>());
}

#[test]
fn startup_waits_for_mounted_current_geometry_and_settled_duel() {
    assert!(!startup_layout_ready(false, 2, 2, true, true, true, false));
    assert!(!startup_layout_ready(true, 1, 2, true, true, true, false));
    assert!(!startup_layout_ready(true, 2, 2, false, true, true, false));
    assert!(!startup_layout_ready(true, 2, 2, true, false, true, false));
    assert!(!startup_layout_ready(true, 2, 2, true, true, false, false));
    assert!(!startup_layout_ready(true, 2, 2, true, true, true, true));
    assert!(startup_layout_ready(true, 2, 2, true, true, true, false));
}

#[test]
fn invalid_or_missing_tracking_never_produces_a_snapshot() {
    let (mut input, anchors) = fixture();
    assert!(
        snapshot(
            None,
            input.monsters.clone(),
            input.spell_traps.clone(),
            input.hand.clone(),
            input.selected
        )
        .is_none()
    );
    assert!(
        snapshot(
            Some(CardTracking::Failed("failure".into())),
            input.monsters.clone(),
            input.spell_traps.clone(),
            input.hand.clone(),
            input.selected
        )
        .is_none()
    );
    input.monsters[6] = None;
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    assert!(targets(&input, &geometry).is_err());
}

#[test]
fn missing_anchors_and_mismatched_card_data_are_errors() {
    let (mut input, _) = fixture();
    assert!(futures::executor::block_on(measure_field(&LayoutAnchors::default())).is_err());
    let (_, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    input.spell_traps[5].as_mut().unwrap().card_code += 1;
    assert!(targets(&input, &geometry).is_err());
}

#[test]
fn unsupported_occupied_field_slots_fail_instead_of_disappearing() {
    let (mut input, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    input.spell_traps[6] = Some(CardData {
        location: CardLocation::SpellTrapZone,
        sequence: 6,
        ..Default::default()
    });
    assert!(targets(&input, &geometry).is_err());
}

#[test]
fn hand_fan_uses_slot_spacing_rotation_and_selected_lift() {
    let base = CardAnchor {
        center_x: 500.0,
        center_y: 600.0,
        width: 100.0,
        height: 146.0,
    };
    let (left, rotation, stack) = hand_position(base, 0, 3, false);
    assert_eq!(rotation, -2.0);
    assert_eq!(stack, 0);
    let shift = 146.0 / 100.0;
    assert!((left.center_x - (422.0 - (-2.0_f64).to_radians().sin() * shift)).abs() < 0.00001);
    assert!((left.center_y - (600.0 + (-2.0_f64).to_radians().cos() * shift)).abs() < 0.00001);
    let (selected, rotation, stack) = hand_position(base, 1, 3, true);
    assert_eq!(rotation, 0.0);
    assert_eq!(stack, 3);
    assert_eq!(selected.center_x, 500.0);
    assert_eq!(selected.center_y, 563.5);
    let resized = CardAnchor {
        width: 200.0,
        height: 292.0,
        ..base
    };
    let (resized_left, _, _) = hand_position(resized, 0, 3, false);
    assert!((resized_left.center_x - (500.0 + 2.0 * (left.center_x - 500.0))).abs() < 0.00001);
}

#[test]
fn duplicate_hand_cards_keep_identity_when_one_moves_and_the_rest_reindex() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{CoreMessage, MoveMessageData};

    let (_, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    let mut snapshot = PreviewSnapshot {
        cards: CardLayer::default(),
        monsters: vec![None; 7],
        spell_traps: vec![None; 8],
        hand: vec![],
        selected: None,
        deck_count: 0,
        extra: vec![],
        graveyard: vec![],
        banishment: vec![],
    };
    for sequence in 0..3 {
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence,
        };
        snapshot.cards.insert(address, 42).unwrap();
        snapshot.hand.push(HandCard {
            index: sequence as usize,
            code: 42,
        });
    }

    fn pile_targets_preserve_ids_counts_and_concealment() {
        let (_, anchors) = fixture();
        let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
        let mut snapshot = PreviewSnapshot {
            cards: CardLayer::default(),
            monsters: vec![None; 7],
            spell_traps: vec![None; 8],
            hand: vec![],
            selected: None,
            deck_count: 2,
            extra: vec![],
            graveyard: vec![],
            banishment: vec![],
        };
        for sequence in 0..2 {
            snapshot
                .cards
                .insert(
                    CardAddress {
                        controller: Player::Player,
                        location: CardLocation::Deck,
                        sequence,
                    },
                    100 + sequence,
                )
                .unwrap();
        }
        let before = targets(&snapshot, &geometry).unwrap();
        assert_eq!(before.len(), 2);
        assert!(before.iter().all(|target| target.appearance
            == CardAppearance::Pile(CardImage::CardBack)
            && !target.interactive));

        use crate::ocgcore::location_info::{
            CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
        };
        use crate::ocgcore::messages::draw::DrawCard;
        use crate::ocgcore::messages::move_card::MoveReason;
        use crate::ocgcore::messages::{CoreMessage, DrawMessageData, MoveMessageData};
        snapshot
            .cards
            .apply(&CoreMessage::Draw(DrawMessageData {
                player: Player::Player,
                cards: vec![DrawCard {
                    code: 101,
                    position: CardPositionFlags::FACE_DOWN_ATTACK,
                }],
            }))
            .unwrap();
        snapshot.deck_count = 1;
        snapshot.hand.push(HandCard {
            index: 0,
            code: 101,
        });
        let drawn = targets(&snapshot, &geometry).unwrap();
        assert_eq!(
            drawn
                .iter()
                .find(|target| target.data.location == CardLocation::Hand)
                .unwrap()
                .id,
            before[1].id
        );
        let loc = |location| CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Known(location),
            sequence: 0,
            position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
        };
        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 101,
                previous: loc(CardLocation::Hand),
                current: loc(CardLocation::Graveyard),
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
        snapshot.hand.clear();
        snapshot.graveyard.push(Some(CardData {
            card_code: 101,
            controller: Player::Player,
            location: CardLocation::Graveyard,
            ..Default::default()
        }));
        let discarded = targets(&snapshot, &geometry).unwrap();
        let grave = discarded
            .iter()
            .find(|target| target.data.location == CardLocation::Graveyard)
            .unwrap();
        assert_eq!(grave.id, before[1].id);
        assert_eq!(
            grave.appearance,
            CardAppearance::Pile(CardImage::Thumbnail(snapshot.graveyard[0].unwrap()))
        );

        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 101,
                previous: loc(CardLocation::Graveyard),
                current: CardLocationInfo {
                    position: CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE),
                    ..loc(CardLocation::Banishment)
                },
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
        snapshot.graveyard.clear();
        snapshot.banishment.push(Some(CardData {
            card_code: 101,
            controller: Player::Player,
            location: CardLocation::Banishment,
            position: Some(BattlePosition::FaceDownDefense),
            ..Default::default()
        }));
        let banished = targets(&snapshot, &geometry).unwrap();
        let banished = banished
            .iter()
            .find(|target| target.data.location == CardLocation::Banishment)
            .unwrap();
        assert_eq!(banished.id, before[1].id);
        assert_eq!(
            banished.appearance,
            CardAppearance::Pile(CardImage::CardBack)
        );
    }

    fn material_targets_follow_host_and_keep_identity_when_detached() {
        use crate::ocgcore::location_info::{
            CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
        };
        use crate::ocgcore::messages::move_card::MoveReason;
        use crate::ocgcore::messages::{CoreMessage, MoveMessageData};
        let (_, anchors) = fixture();
        let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
        let mut snapshot = PreviewSnapshot {
            cards: CardLayer::default(),
            monsters: vec![None; 7],
            spell_traps: vec![None; 8],
            hand: vec![],
            selected: None,
            deck_count: 0,
            extra: vec![],
            graveyard: vec![],
            banishment: vec![],
        };
        let address = |location| CardAddress {
            controller: Player::Player,
            location,
            sequence: 0,
        };
        snapshot
            .cards
            .insert(address(CardLocation::MonsterZone), 10)
            .unwrap();
        let material_id = snapshot
            .cards
            .insert(address(CardLocation::Hand), 20)
            .unwrap();
        snapshot.monsters[0] = Some(CardData {
            card_code: 10,
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            ..Default::default()
        });
        let ordinary = |location| CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Known(location),
            sequence: 0,
            position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
        };
        let overlay = CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Overlay {
                host: Some(CardLocation::MonsterZone),
            },
            sequence: 0,
            position: CardLocationPosition::OverlaySequence(0),
        };
        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 20,
                previous: ordinary(CardLocation::Hand),
                current: overlay,
                reason: MoveReason::XYZ | MoveReason::MATERIAL,
            }))
            .unwrap();
        let attached = targets(&snapshot, &geometry).unwrap();
        let material = attached
            .iter()
            .find(|target| target.id == material_id)
            .unwrap();
        assert_eq!(material.appearance, CardAppearance::Material);
        assert_eq!(material.data.card_code, 20);
        assert!(!material.interactive);
        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 20,
                previous: overlay,
                current: ordinary(CardLocation::Graveyard),
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
        snapshot.graveyard.push(Some(CardData {
            card_code: 20,
            controller: Player::Player,
            location: CardLocation::Graveyard,
            ..Default::default()
        }));
        let detached = targets(&snapshot, &geometry).unwrap();
        assert_eq!(
            detached
                .iter()
                .find(|target| target.data.location == CardLocation::Graveyard)
                .unwrap()
                .id,
            material_id
        );
    }
    pile_targets_preserve_ids_counts_and_concealment();
    material_targets_follow_host_and_keep_identity_when_detached();
    let before = targets(&snapshot, &geometry).unwrap();
    assert_eq!(before.len(), 3);
    assert_ne!(before[0].id, before[1].id);
    assert_ne!(before[1].id, before[2].id);
    let loc = |location, sequence| CardLocationInfo {
        controller: Some(Player::Player),
        location: CardLocationKind::Known(location),
        sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    };
    snapshot
        .cards
        .apply(&CoreMessage::Move(MoveMessageData {
            card_code: 42,
            previous: loc(CardLocation::Hand, 0),
            current: loc(CardLocation::MonsterZone, 0),
            reason: MoveReason::EFFECT,
        }))
        .unwrap();
    snapshot.hand = vec![
        HandCard { index: 0, code: 42 },
        HandCard { index: 1, code: 42 },
    ];
    snapshot.monsters[0] = Some(CardData {
        card_code: 42,
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
        ..Default::default()
    });
    let after = targets(&snapshot, &geometry).unwrap();
    assert_eq!(after[0].id, before[0].id);
    assert_eq!(after[1].id, before[1].id);
    assert_eq!(after[2].id, before[2].id);
    assert_eq!(after[1].data.sequence, 0);
    assert_eq!(after[2].data.sequence, 1);
    assert_eq!(after[1].rotation, -1.0);
    assert_eq!(after[2].rotation, 1.0);
}

#[test]
fn incoming_graveyard_cards_do_not_replace_the_stationary_pile_image() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{CoreMessage, MoveMessageData};
    let (_, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    let mut snapshot = PreviewSnapshot {
        cards: CardLayer::default(),
        monsters: vec![None; 7],
        spell_traps: vec![None; 8],
        hand: vec![HandCard { index: 0, code: 20 }],
        selected: None,
        deck_count: 0,
        extra: vec![],
        graveyard: vec![Some(CardData {
            card_code: 10,
            controller: Player::Player,
            location: CardLocation::Graveyard,
            ..Default::default()
        })],
        banishment: vec![],
    };
    let address = |location, sequence| CardAddress {
        controller: Player::Player,
        location,
        sequence,
    };
    let stationary = snapshot
        .cards
        .insert(address(CardLocation::Graveyard, 0), 10)
        .unwrap();
    let moving = snapshot
        .cards
        .insert(address(CardLocation::Hand, 0), 20)
        .unwrap();
    let mut playback = CardPlayback::default();
    playback.begin(
        vec![moving],
        snapshot.cards.clone(),
        [vec![], snapshot.graveyard.clone(), vec![]],
    );
    let loc = |location, sequence| CardLocationInfo {
        controller: Some(Player::Player),
        location: CardLocationKind::Known(location),
        sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    };
    snapshot
        .cards
        .apply(&CoreMessage::Move(MoveMessageData {
            card_code: 20,
            previous: loc(CardLocation::Hand, 0),
            current: loc(CardLocation::Graveyard, 1),
            reason: MoveReason::EFFECT,
        }))
        .unwrap();
    snapshot.hand.clear();
    snapshot.graveyard.push(Some(CardData {
        card_code: 20,
        controller: Player::Player,
        location: CardLocation::Graveyard,
        sequence: 1,
        ..Default::default()
    }));
    let mut rendered = targets(&snapshot, &geometry).unwrap();
    preserve_pile_images(&snapshot, &mut rendered, &playback).unwrap();
    assert_eq!(
        rendered
            .iter()
            .find(|target| target.id == stationary)
            .unwrap()
            .appearance,
        CardAppearance::Pile(CardImage::Thumbnail(snapshot.graveyard[0].unwrap()))
    );
    assert_eq!(
        rendered
            .iter()
            .find(|target| target.id == moving)
            .unwrap()
            .appearance,
        CardAppearance::Pile(CardImage::Thumbnail(CardData {
            card_code: 20,
            controller: Player::Player,
            location: CardLocation::Graveyard,
            sequence: 1,
            ..Default::default()
        }))
    );
    playback.finish(playback.generation);
    let mut completed = targets(&snapshot, &geometry).unwrap();
    preserve_pile_images(&snapshot, &mut completed, &playback).unwrap();
    assert_eq!(
        completed
            .iter()
            .find(|target| target.id == stationary)
            .unwrap()
            .appearance,
        CardAppearance::Pile(CardImage::Thumbnail(snapshot.graveyard[1].unwrap()))
    );
}
