use super::super::layout_anchors::{Anchor, AnchorPart};
use super::*;
use dioxus::html::geometry::{PixelsRect, PixelsSize, euclid::point2};
use dioxus::html::{MountedResult, RenderedElementBacking};
use std::any::Any;
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
fn invalid_or_missing_tracking_never_enables_the_preview() {
    let (mut input, anchors) = fixture();
    assert!(
        snapshot(
            false,
            Some(CardTracking::Active(input.cards.clone())),
            input.monsters.clone(),
            input.spell_traps.clone(),
            input.hand.clone(),
            input.selected
        )
        .is_none()
    );
    assert!(
        snapshot(
            true,
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
