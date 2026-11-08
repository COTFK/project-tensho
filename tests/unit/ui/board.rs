use super::super::card_layout::{CardLayoutSnapshot, CardRenderMode};
use super::super::layout_anchors::{AnchorGeometry, LayoutAnchorId};
use super::*;
use crate::ocgcore::DuelCardData;
use dioxus::html::geometry::{PixelsRect, PixelsSize, euclid::point2};
use dioxus::html::{MountedResult, RenderedElementBacking};
use std::any::Any;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

struct TestElement(PixelsRect);

fn set_data(cards: &mut CardTracker, address: CardAddress, data: DuelCardData) {
    cards.update_queried_data(address, data).unwrap();
}

impl RenderedElementBacking for TestElement {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        let rect = self.0;
        Box::pin(async move { Ok(rect) })
    }
}

fn fixture() -> (CardLayoutSnapshot, LayoutAnchorRegistry) {
    let mut snapshot = CardLayoutSnapshot {
        cards: CardTracker::default(),
        selected: None,
    };
    let anchors = LayoutAnchorRegistry::default();
    anchors.insert(
        LayoutAnchorId::HandBase,
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
            LayoutAnchorId::Pile(location),
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
                controller: Player::Local,
                location,
                sequence,
            };
            let code = 100 + sequence;
            snapshot.cards.insert(address, code).unwrap();
            let data = DuelCardData {
                card_code: code,
                controller: Player::Local,
                location,
                sequence: u8::try_from(sequence).unwrap(),
                ..Default::default()
            };
            set_data(&mut snapshot.cards, address, data);
            let rect = PixelsRect::new(
                point2(f64::from(sequence) * 100.0, 20.0),
                PixelsSize::new(59.0, 86.0),
            );
            anchors.insert(
                LayoutAnchorId::Zone(location, sequence),
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
            target.geometry.center_x,
            f64::from(target.data.sequence) * 100.0 + 29.5
        );
        assert_eq!(target.geometry.center_y, 63.0);
        assert_eq!(target.rotation, 0.0);
        assert_eq!(target.render_mode, CardRenderMode::Interactive);
    }
}

#[test]
fn measurement_order_and_keyed_geometry_ignore_registration_order() {
    use std::cell::RefCell;

    struct RecordingElement {
        anchor: LayoutAnchorId,
        rect: PixelsRect,
        calls: Rc<RefCell<Vec<LayoutAnchorId>>>,
    }

    impl RenderedElementBacking for RecordingElement {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
            self.calls.borrow_mut().push(self.anchor);
            let rect = self.rect;
            Box::pin(async move { Ok(rect) })
        }
    }

    let expected: Vec<_> = [
        (CardLocation::MonsterZone, 7),
        (CardLocation::SpellTrapZone, 6),
    ]
    .into_iter()
    .flat_map(|(location, count)| {
        (0..count).map(move |index| LayoutAnchorId::Zone(location, index))
    })
    .chain([
        LayoutAnchorId::Pile(CardLocation::Deck),
        LayoutAnchorId::Pile(CardLocation::ExtraDeck),
        LayoutAnchorId::Pile(CardLocation::Graveyard),
        LayoutAnchorId::Pile(CardLocation::Banishment),
        LayoutAnchorId::HandBase,
    ])
    .collect();
    assert_eq!(expected.len(), 18);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let anchors = LayoutAnchorRegistry::default();
    for (index, &anchor) in expected.iter().enumerate().rev() {
        let index = index as f64;
        anchors.insert(
            anchor,
            Rc::new(()),
            Rc::new(MountedData::new(RecordingElement {
                anchor,
                rect: PixelsRect::new(
                    point2(100.0 * index, 20.0 + index),
                    PixelsSize::new(59.0 + index, 86.0 + index),
                ),
                calls: calls.clone(),
            })),
        );
    }
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    assert_eq!(*calls.borrow(), expected);
    assert_eq!(geometry.anchors.len(), 17);
    for (index, anchor) in expected.iter().enumerate() {
        let index = index as f64;
        let expected_rect = AnchorGeometry {
            center_x: 100.0 * index + (59.0 + index) / 2.0,
            center_y: 20.0 + index + (86.0 + index) / 2.0,
            width: 59.0 + index,
            height: 86.0 + index,
        };
        assert_eq!(
            if *anchor == LayoutAnchorId::HandBase {
                geometry.hand_base
            } else {
                geometry.anchors[anchor]
            },
            expected_rect
        );
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
        LayoutAnchorId::Zone(CardLocation::MonsterZone, 0),
        Rc::new(()),
        Rc::new(MountedData::new(DelayedElement(Cell::new(0)))),
    );
    assert!(futures::executor::block_on(measure_field_when_ready(&anchors)).is_ok());
}

#[test]
fn persistent_missing_anchors_are_reported() {
    let error = match futures::executor::block_on(measure_field_when_ready(
        &LayoutAnchorRegistry::default(),
    )) {
        Ok(_) => panic!("missing anchors should fail"),
        Err(error) => error,
    };
    assert!(error.is::<AnchorNotReady>());
}

#[test]
fn layout_readiness_requires_mounted_current_geometry_targets_and_artwork() {
    let (snapshot, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    let targets = Some(Ok(targets(&snapshot, &geometry).unwrap()));
    let measured = Some((2, Ok(geometry)));
    for (mounted, revision, measured, targets, artwork_ready) in [
        (false, 2, measured.as_ref(), targets.as_ref(), true),
        (true, 3, measured.as_ref(), targets.as_ref(), true),
        (true, 2, None, targets.as_ref(), true),
        (true, 2, measured.as_ref(), None, true),
        (true, 2, measured.as_ref(), targets.as_ref(), false),
    ] {
        assert_eq!(
            layout_status(
                mounted,
                revision,
                measured,
                targets,
                (None, artwork_ready),
                true
            ),
            LayoutStatus::Pending
        );
    }
    assert_eq!(
        layout_status(
            true,
            2,
            measured.as_ref(),
            targets.as_ref(),
            (None, true),
            true
        ),
        LayoutStatus::Ready
    );
}

#[test]
fn stale_measurement_errors_are_ignored_but_current_errors_are_reported() {
    let measured = Some((1, Err("missing anchor".into())));
    assert_eq!(
        layout_status(true, 2, measured.as_ref(), None, (None, true), true),
        LayoutStatus::Pending
    );
    for (preloading, action) in [(true, "measure"), (false, "remeasure")] {
        assert_eq!(
            layout_status(true, 1, measured.as_ref(), None, (None, true), preloading),
            LayoutStatus::Failed(format!("Cannot {action} the field: missing anchor"))
        );
    }
}

#[test]
fn target_and_startup_artwork_errors_preserve_failure_priority() {
    let measured = Some((1, Err("missing anchor".into())));
    let targets = Some(Err("invalid target".into()));
    let artwork = (Some("failed image".into()), false);
    assert_eq!(
        layout_status(
            true,
            1,
            measured.as_ref(),
            targets.as_ref(),
            artwork.clone(),
            true
        ),
        LayoutStatus::Failed("invalid target".into())
    );
    assert_eq!(
        layout_status(true, 1, measured.as_ref(), None, artwork.clone(), true),
        LayoutStatus::Failed("Cannot load card artwork: failed image".into())
    );
    assert_eq!(
        layout_status(true, 1, measured.as_ref(), None, artwork, false),
        LayoutStatus::Failed("Cannot remeasure the field: missing anchor".into())
    );
}

#[test]
fn artwork_failures_after_preloading_do_not_stop_a_ready_layout() {
    let (snapshot, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    let targets = Some(Ok(targets(&snapshot, &geometry).unwrap()));
    let measured = Some((1, Ok(geometry)));
    assert_eq!(
        layout_status(
            true,
            1,
            measured.as_ref(),
            targets.as_ref(),
            (Some("failed image".into()), false),
            false,
        ),
        LayoutStatus::Ready
    );
}

#[test]
fn duel_board_reports_errors_without_owning_duel_lifecycle() {
    use crate::state::DuelState;
    use crate::ui::playback::{DuelPlayback, use_playback};
    use crate::ui::view_state::DuelViewState;
    use std::cell::RefCell;

    thread_local! {
        static REPORTED: RefCell<Vec<LayoutStatus>> = const { RefCell::new(Vec::new()) };
        static CONTEXT: RefCell<Option<(DuelState, DuelPlayback)>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let mut state = use_context_provider(DuelState::default);
        let view = use_context_provider(DuelViewState::default);
        let mut playback = use_playback();
        let controller = use_signal(|| None::<crate::duel::controller::DuelController>);
        super::super::use_duel_contexts(state, view, playback, controller.into());
        use_context_provider(LayoutAnchorRegistry::default);
        use_context_provider(|| LayoutRevision(Signal::new(0)));
        use_hook(move || {
            state.failure.set(Some("existing failure".into()));
            state.restarting.set(true);
            playback
                .animation
                .write()
                .begin(vec![], CardTracker::default());
        });
        CONTEXT.with(|slot| *slot.borrow_mut() = Some((state, playback)));
        let tracking = use_signal(|| Some(CardTracker::default()));
        let selected = use_signal(|| None::<CardInstanceId>);
        rsx! {
            DuelBoard {
                tracking,
                selected,
                animation: playback.animation,
                preloading: true,
                effects_open: false,
                deck_codes: vec![],
                onlayout: move |status| REPORTED.with(|reports| reports.borrow_mut().push(status)),
            }
        }
    }

    REPORTED.with(|reports| reports.borrow_mut().clear());
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let reporting = async {
        for _ in 0..32 {
            dom.process_events();
            if REPORTED.with(|reports| {
                reports
                    .borrow()
                    .iter()
                    .any(|status| matches!(status, LayoutStatus::Failed(_)))
            }) {
                return;
            }
            dom.wait_for_work().await;
            dom.render_immediate_to_vec();
        }
        panic!("Layout did not report its measurement failure");
    };
    match futures::executor::block_on(futures::future::select(
        Box::pin(reporting),
        Box::pin(wait_millis(5_000)),
    )) {
        futures::future::Either::Left(_) => {}
        futures::future::Either::Right(_) => panic!("Timed out waiting for the layout report"),
    }
    REPORTED.with(|reports| {
        assert_eq!(reports.borrow().first(), Some(&LayoutStatus::Pending));
        assert!(matches!(
            reports.borrow().last(),
            Some(LayoutStatus::Failed(error)) if error.starts_with("Cannot measure the field:")
        ));
        reports.borrow_mut().clear();
    });
    CONTEXT.with(|slot| {
        let (state, playback) = slot.borrow_mut().take().unwrap();
        assert_eq!(state.failure.peek().as_deref(), Some("existing failure"));
        assert!(*state.restarting.peek());
        assert!(playback.animation.peek().is_active());
    });
}

#[test]
fn deck_artwork_includes_unique_faces_thumbnails_and_backs() {
    let urls = deck_artwork_urls(&[42, 42, 100]);
    assert_eq!(
        urls,
        BTreeSet::from([
            card_image_url(42),
            card_thumbnail_url(42),
            card_image_url(100),
            card_thumbnail_url(100),
            CardImage::CardBack.url(),
            CardImage::ExtraDeckBack.url(),
        ])
    );
}

#[test]
fn artwork_readiness_waits_for_every_image_and_reports_failures() {
    let urls = deck_artwork_urls(&[42, 100]);
    let mut artwork = HashMap::new();
    for url in &urls {
        assert!(!artwork_status(&urls, &artwork).1);
        artwork.insert(url.clone(), ArtworkStatus::Loaded);
    }
    assert_eq!(artwork_status(&urls, &artwork), (None, true));

    let failed = card_image_url(100);
    artwork.insert(failed.clone(), ArtworkStatus::Failed);
    assert_eq!(artwork_status(&urls, &artwork), (Some(failed), false));
}

#[test]
fn preloaded_deck_images_remain_mounted_after_startup() {
    use dioxus::core::{AttributeValue, Mutation};
    use std::cell::RefCell;

    thread_local! {
        static URLS: RefCell<Option<Signal<BTreeSet<String>>>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let urls = use_signal(|| {
            let mut urls = deck_artwork_urls(&[42]);
            urls.insert(card_image_url(100));
            urls
        });
        URLS.with(|slot| *slot.borrow_mut() = Some(urls));
        let artwork = use_signal(HashMap::<String, ArtworkStatus>::new);
        rsx! { ArtworkPreloader { urls: urls(), artwork } }
    }

    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let retained = deck_artwork_urls(&[42]);
    let image_ids: Vec<_> = initial
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "src",
                value: AttributeValue::Text(url),
                id,
                ..
            } if retained.contains(url) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(image_ids.len(), retained.len(), "{:?}", initial.edits);

    URLS.with(|slot| slot.borrow().unwrap().set(retained));
    let after_startup = dom.render_immediate_to_vec();
    assert!(
        !after_startup.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if image_ids.contains(id)
        )),
        "{:?}",
        after_startup.edits
    );
    URLS.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn missing_anchors_are_errors() {
    let (mut input, _) = fixture();
    assert!(futures::executor::block_on(measure_field(&LayoutAnchorRegistry::default())).is_err());
    let (_, anchors) = fixture();
    let geometry = futures::executor::block_on(measure_field(&anchors)).unwrap();
    let address = CardAddress {
        controller: Player::Local,
        location: CardLocation::SpellTrapZone,
        sequence: 5,
    };
    let data = input.cards.card_at(address).unwrap().data.unwrap();
    set_data(&mut input.cards, address, data);
    assert!(targets(&input, &geometry).is_ok());
}
