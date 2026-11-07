use super::*;
use crate::ui::card_layer::CardLayer;

#[test]
fn interaction_gate_removes_inert_when_playback_finishes() {
    use dioxus::core::{AttributeValue, Mutation};
    use std::cell::RefCell;
    thread_local! {
        static ACTIVE: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
    }
    fn app() -> Element {
        let active = use_signal(|| false);
        ACTIVE.with(|slot| *slot.borrow_mut() = Some(active));
        rsx! { InteractionGate { active: active(), button { "Action" } } }
    }
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    assert!(!initial.edits.iter().any(|edit| matches!(
        edit,
        Mutation::SetAttribute {
            name: "inert",
            value: AttributeValue::Text(_),
            ..
        }
    )));
    ACTIVE.with(|slot| slot.borrow().unwrap().set(true));
    let blocked = dom.render_immediate_to_vec();
    assert!(
        blocked.edits.iter().any(|edit| matches!(
            edit,
            Mutation::SetAttribute {
                name: "inert",
                value: AttributeValue::Text(_),
                ..
            }
        )),
        "{:?}",
        blocked.edits
    );
    ACTIVE.with(|slot| slot.borrow().unwrap().set(false));
    let unblocked = dom.render_immediate_to_vec();
    assert!(
        unblocked.edits.iter().any(|edit| matches!(
            edit,
            Mutation::SetAttribute {
                name: "inert",
                value: AttributeValue::None,
                ..
            }
        )),
        "{:?}",
        unblocked.edits
    );
    ACTIVE.with(|slot| *slot.borrow_mut() = None);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn replaces_tracking_and_ui_state_when_a_duel_restarts() {
    fn app() -> Element {
        let core = futures::executor::block_on(OCGCore::load(
            |_| panic!("Unexpected card lookup"),
            Box::new(|_| None),
            |_, _| {},
        ))
        .unwrap();
        let first = core.create_duel(0).unwrap();
        let second = core.create_duel(0).unwrap();
        let mut state = use_context_provider(UIState::default);
        let mut duel = use_context_provider(|| Signal::new(None::<Duel>));
        let mut tracking = use_context_provider(|| Signal::new(None::<CardTracking>));
        let initial = DuelSetup {
            duel: first.clone(),
            cards: CardLayer::from_decks(&[1, 2], &[]).unwrap(),
        };
        install_duel(&mut state, &mut duel, &mut tracking, Some(initial.clone()));
        state.waiting_on_input.set(true);
        tracking.set(Some(CardTracking::Failed("test failure".into())));
        install_duel(&mut state, &mut duel, &mut tracking, Some(initial));
        assert!((state.waiting_on_input)());
        assert!(matches!(&*tracking.read(), Some(CardTracking::Failed(_))));
        let replacement = DuelSetup {
            duel: second.clone(),
            cards: CardLayer::from_decks(&[3], &[]).unwrap(),
        };
        install_duel(&mut state, &mut duel, &mut tracking, Some(replacement));
        assert!(!(state.waiting_on_input)());
        assert!(duel.read().as_ref().unwrap().ptr_eq(&second));
        let guard = tracking.read();
        let Some(CardTracking::Active(layer)) = &*guard else {
            panic!("Restart did not initialize a fresh tracker");
        };
        assert_eq!(layer.cards().len(), 1);
        assert_eq!(layer.cards()[0].code, 3);
        drop(guard);
        install_duel(&mut state, &mut duel, &mut tracking, None);
        assert!(duel.read().is_none());
        assert!(tracking.read().is_none());
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
