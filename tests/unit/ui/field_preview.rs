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
    };
    let anchors = LayoutAnchors::default();
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
            input.spell_traps.clone()
        )
        .is_none()
    );
    assert!(
        snapshot(
            true,
            Some(CardTracking::Failed("failure".into())),
            input.monsters.clone(),
            input.spell_traps.clone()
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
