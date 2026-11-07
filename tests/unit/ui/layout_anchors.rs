use super::*;
use dioxus::html::geometry::{PixelsSize, euclid::point2};
use dioxus::html::{MountedResult, RenderedElementBacking};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;

struct TestElement(Rc<RefCell<PixelsRect>>);

impl RenderedElementBacking for TestElement {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        let rect = *self.0.borrow();
        Box::pin(async move { Ok(rect) })
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> PixelsRect {
    PixelsRect::new(point2(x, y), PixelsSize::new(width, height))
}

fn address(location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller: Player::Player,
        location,
        sequence,
    }
}

fn mount(anchors: &LayoutAnchors, key: AnchorKey, rect: Rc<RefCell<PixelsRect>>) -> Rc<()> {
    let owner = Rc::new(());
    anchors.insert(
        key,
        owner.clone(),
        Rc::new(MountedData::new(TestElement(rect))),
    );
    owner
}

#[test]
fn address_mapping_covers_zones_hand_and_piles() {
    assert_eq!(
        Anchor::try_from(address(CardLocation::MonsterZone, 6)).unwrap(),
        Anchor::Zone(CardLocation::MonsterZone, 6)
    );
    assert_eq!(
        Anchor::try_from(address(CardLocation::SpellTrapZone, 5)).unwrap(),
        Anchor::Zone(CardLocation::SpellTrapZone, 5)
    );
    assert!(Anchor::try_from(address(CardLocation::MonsterZone, 7)).is_err());
    assert!(Anchor::try_from(address(CardLocation::Hand, 257)).is_err());
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        assert_eq!(
            Anchor::try_from(address(location, 23)).unwrap(),
            Anchor::Pile(location)
        );
    }
    let mut opponent = address(CardLocation::Hand, 0);
    opponent.controller = Player::Opponent;
    assert!(Anchor::try_from(opponent).is_err());
}

#[test]
fn hand_base_measurements_use_current_viewport() {
    let anchors = LayoutAnchors::default();
    let base = Rc::new(RefCell::new(rect(70.0, 100.0, 59.0, 86.0)));
    mount(&anchors, (Anchor::HandBase, AnchorPart::Card), base.clone());
    let measure = || futures::executor::block_on(anchors.measure_hand_base()).unwrap();
    assert_eq!(
        measure(),
        CardAnchor {
            center_x: 99.5,
            center_y: 143.0,
            width: 59.0,
            height: 86.0
        }
    );
    *base.borrow_mut() = rect(140.0, 200.0, 118.0, 172.0);
    assert_eq!(
        measure(),
        CardAnchor {
            center_x: 199.0,
            center_y: 286.0,
            width: 118.0,
            height: 172.0
        }
    );
}

#[test]
fn field_and_pile_measurements_use_card_rect_and_ignore_pile_sequence() {
    let anchors = LayoutAnchors::default();
    for anchor in [
        Anchor::Zone(CardLocation::MonsterZone, 0),
        Anchor::Pile(CardLocation::Deck),
    ] {
        mount(
            &anchors,
            (anchor, AnchorPart::Card),
            Rc::new(RefCell::new(rect(10.0, 20.0, 59.0, 86.0))),
        );
    }
    for address in [
        address(CardLocation::MonsterZone, 0),
        address(CardLocation::Deck, 45),
    ] {
        let measured = futures::executor::block_on(anchors.measure(address)).unwrap();
        assert_eq!(
            measured,
            CardAnchor {
                center_x: 39.5,
                center_y: 63.0,
                width: 59.0,
                height: 86.0
            }
        );
    }
}

#[test]
fn replacement_and_unmount_do_not_leave_stale_anchors() {
    let anchors = LayoutAnchors::default();
    let key = (Anchor::HandBase, AnchorPart::Card);
    let old = mount(
        &anchors,
        key,
        Rc::new(RefCell::new(rect(0.0, 0.0, 59.0, 86.0))),
    );
    let new = mount(
        &anchors,
        key,
        Rc::new(RefCell::new(rect(0.0, 0.0, 118.0, 172.0))),
    );
    anchors.remove(key, &old);
    assert!(anchors.0.borrow().contains_key(&key));
    anchors.remove(key, &new);
    assert!(!anchors.0.borrow().contains_key(&key));
    assert!(futures::executor::block_on(anchors.measure_hand_base()).is_err());
}

#[test]
fn zero_sized_and_non_finite_measurements_are_rejected() {
    assert!(CardAnchor::from_rects(rect(0.0, 0.0, 0.0, 0.0), rect(0.0, 0.0, 0.0, 0.0)).is_err());
    assert!(
        CardAnchor::from_rects(rect(f64::NAN, 0.0, 59.0, 86.0), rect(0.0, 0.0, 59.0, 86.0))
            .is_err()
    );
}
