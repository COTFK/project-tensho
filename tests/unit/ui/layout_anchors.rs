use super::*;
use dioxus::html::geometry::{PixelsSize, euclid::point2};
use dioxus::html::{MountedResult, RenderedElementBacking};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;

struct TestElement(Rc<RefCell<PixelsRect>>);

struct DeferredElement(RefCell<Option<futures::channel::oneshot::Receiver<PixelsRect>>>);

impl RenderedElementBacking for DeferredElement {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        let receiver = self.0.borrow_mut().take().unwrap();
        Box::pin(async move { Ok(receiver.await.unwrap()) })
    }
}

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
        controller: Player::Local,
        location,
        sequence,
    }
}

fn mount(
    anchors: &LayoutAnchorRegistry,
    anchor: LayoutAnchorId,
    rect: Rc<RefCell<PixelsRect>>,
) -> Rc<()> {
    let owner = Rc::new(());
    anchors.insert(
        anchor,
        owner.clone(),
        Rc::new(MountedData::new(TestElement(rect))),
    );
    owner
}

#[test]
fn address_mapping_covers_zones_hand_and_piles() {
    assert_eq!(
        LayoutAnchorId::try_from(address(CardLocation::MonsterZone, 6)).unwrap(),
        LayoutAnchorId::Zone(CardLocation::MonsterZone, 6)
    );
    assert_eq!(
        LayoutAnchorId::try_from(address(CardLocation::SpellTrapZone, 5)).unwrap(),
        LayoutAnchorId::Zone(CardLocation::SpellTrapZone, 5)
    );
    assert!(LayoutAnchorId::try_from(address(CardLocation::MonsterZone, 7)).is_err());
    assert!(LayoutAnchorId::try_from(address(CardLocation::Hand, 257)).is_err());
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        assert_eq!(
            LayoutAnchorId::try_from(address(location, 23)).unwrap(),
            LayoutAnchorId::Pile(location)
        );
    }
    let mut opponent = address(CardLocation::Hand, 0);
    opponent.controller = Player::Opponent;
    assert!(LayoutAnchorId::try_from(opponent).is_err());
}

#[test]
fn hand_base_measurements_use_current_viewport() {
    let anchors = LayoutAnchorRegistry::default();
    let base = Rc::new(RefCell::new(rect(70.0, 100.0, 59.0, 86.0)));
    mount(&anchors, LayoutAnchorId::HandBase, base.clone());
    let measure = || futures::executor::block_on(anchors.measure_hand_base()).unwrap();
    assert_eq!(
        measure(),
        AnchorGeometry {
            center_x: 99.5,
            center_y: 143.0,
            width: 59.0,
            height: 86.0
        }
    );
    *base.borrow_mut() = rect(140.0, 200.0, 118.0, 172.0);
    assert_eq!(
        measure(),
        AnchorGeometry {
            center_x: 199.0,
            center_y: 286.0,
            width: 118.0,
            height: 172.0
        }
    );
}

#[test]
fn field_and_pile_measurements_use_card_rect_and_ignore_pile_sequence() {
    let anchors = LayoutAnchorRegistry::default();
    for anchor in [
        LayoutAnchorId::Zone(CardLocation::MonsterZone, 0),
        LayoutAnchorId::Pile(CardLocation::Deck),
    ] {
        mount(
            &anchors,
            anchor,
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
            AnchorGeometry {
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
    let anchors = LayoutAnchorRegistry::default();
    let key = LayoutAnchorId::HandBase;
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
    for rect in [rect(0.0, 0.0, 0.0, 86.0), rect(0.0, 0.0, 59.0, 0.0)] {
        let error = AnchorGeometry::from_rect(rect).unwrap_err();
        assert!(error.is::<AnchorNotReady>());
        assert_eq!(error.to_string(), "Card anchor has no size yet");
    }
    for rect in [
        rect(f64::NAN, 0.0, 59.0, 86.0),
        rect(0.0, f64::INFINITY, 59.0, 86.0),
        rect(0.0, 0.0, f64::INFINITY, 86.0),
        rect(0.0, 0.0, 59.0, f64::NAN),
        rect(0.0, 0.0, -59.0, 86.0),
        rect(0.0, 0.0, 59.0, -86.0),
    ] {
        let error = AnchorGeometry::from_rect(rect).unwrap_err();
        assert!(!error.is::<AnchorNotReady>());
        assert_eq!(error.to_string(), "Invalid or hidden card anchor");
    }
}

#[test]
fn missing_measurements_preserve_anchor_specific_retry_errors() {
    let anchors = LayoutAnchorRegistry::default();
    let hand = futures::executor::block_on(anchors.measure_hand_base()).unwrap_err();
    assert!(hand.is::<AnchorNotReady>());
    assert_eq!(hand.to_string(), "Hand anchor not mounted");
    for location in [CardLocation::MonsterZone, CardLocation::Deck] {
        let card = futures::executor::block_on(anchors.measure(address(location, 0))).unwrap_err();
        assert!(card.is::<AnchorNotReady>());
        assert_eq!(card.to_string(), "Card anchor not mounted");
    }
}

#[test]
fn replacement_or_unmount_while_measuring_rejects_stale_rectangles() {
    for anchor in [
        LayoutAnchorId::HandBase,
        LayoutAnchorId::Zone(CardLocation::MonsterZone, 0),
        LayoutAnchorId::Pile(CardLocation::Deck),
    ] {
        for replace in [false, true] {
            let anchors = LayoutAnchorRegistry::default();
            let owner = Rc::new(());
            let (sender, receiver) = futures::channel::oneshot::channel();
            anchors.insert(
                anchor,
                owner.clone(),
                Rc::new(MountedData::new(DeferredElement(RefCell::new(Some(
                    receiver,
                ))))),
            );
            let mut measurement = Box::pin(async {
                match anchor {
                    LayoutAnchorId::HandBase => anchors.measure_hand_base().await,
                    LayoutAnchorId::Zone(location, sequence) => {
                        anchors.measure(address(location, sequence)).await
                    }
                    LayoutAnchorId::Pile(location) => anchors.measure(address(location, 23)).await,
                }
            });
            let mut context = std::task::Context::from_waker(futures::task::noop_waker_ref());
            assert!(measurement.as_mut().poll(&mut context).is_pending());
            if replace {
                mount(
                    &anchors,
                    anchor,
                    Rc::new(RefCell::new(rect(100.0, 200.0, 118.0, 172.0))),
                );
            } else {
                anchors.remove(anchor, &owner);
            }
            sender.send(rect(0.0, 0.0, 59.0, 86.0)).unwrap();
            let error = futures::executor::block_on(measurement).unwrap_err();
            assert!(!error.is::<AnchorNotReady>());
            assert_eq!(
                error.to_string(),
                if anchor == LayoutAnchorId::HandBase {
                    "Hand anchor changed while measuring"
                } else {
                    "Card anchor changed while measuring"
                }
            );
        }
    }
}
