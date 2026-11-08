use super::*;
use crate::duel::cards::CardTracker;
use crate::duel::input::PendingInput;
use crate::ocgcore::CardData;

#[path = "events.rs"]
mod events;

#[test]
fn restart_button_cancels_playback_and_clears_pending_input_and_view() {
    use crate::duel::cards::CardAddress;
    use crate::ocgcore::constants::{CardLocation, Player};
    use crate::ocgcore::messages::CoreMessage;
    use crate::ui::card_animation::AnimationPhase;
    use dioxus::core::{AttributeValue, Mutation};
    use std::cell::RefCell;

    thread_local! {
        static STATE: RefCell<Option<UIState>> = const { RefCell::new(None) };
        static VIEW: RefCell<Option<ViewState>> = const { RefCell::new(None) };
        static ANIMATION: RefCell<Option<Signal<CardAnimationState>>> = const { RefCell::new(None) };
        static CHAIN: RefCell<Option<Signal<ChainPlayback>>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        use_context_provider(ViewState::default);
        use_context_provider(|| {
            let mut state = UIState::default();
            state.restarting.set(true);
            state
        });
        let duel_resource = use_resource(std::future::pending::<anyhow::Result<DuelSetup>>);
        let screen = DuelScreen(DuelScreenProps {
            duel_resource,
            deck_codes: vec![],
        });
        STATE.with(|slot| *slot.borrow_mut() = Some(consume_context::<UIState>()));
        VIEW.with(|slot| *slot.borrow_mut() = Some(consume_context::<ViewState>()));
        ANIMATION.with(|slot| {
            *slot.borrow_mut() = Some(consume_context::<Signal<CardAnimationState>>())
        });
        CHAIN.with(|slot| *slot.borrow_mut() = Some(consume_context::<Signal<ChainPlayback>>()));
        screen
    }

    events::install();
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let restart = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "aria-label",
                value: AttributeValue::Text(label),
                id,
                ..
            } if *label == "Restart game" => Some(*id),
            _ => None,
        })
        .expect("Restart button should be mounted");
    dom.process_events();
    let mut state = STATE.with(|slot| slot.borrow().unwrap());
    let mut view = VIEW.with(|slot| slot.borrow().unwrap());
    let mut animation = ANIMATION.with(|slot| slot.borrow().unwrap());
    let mut chain = CHAIN.with(|slot| slot.borrow().unwrap());
    let mut cards = CardTracker::default();
    let card = cards
        .insert(
            CardAddress {
                controller: Player::Player,
                location: CardLocation::MonsterZone,
                sequence: 0,
            },
            1,
        )
        .unwrap();
    state.cards.set(Some(cards.clone()));
    view.selected_card.set(Some(card));
    view.open_pile.set(Some(CardLocation::Graveyard));
    view.effects_to_select_from.set(vec![CardData {
        card_code: 1,
        ..Default::default()
    }]);
    state
        .pending_input
        .set(Some(PendingInput::YesNo("Activate?".into())));
    animation.write().begin(vec![card], cards.clone());
    let message = CoreMessage::try_from(
        [
            33, 0, 0, 0, 70, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0,
        ]
        .as_slice(),
    )
    .unwrap();
    chain.write().record(&message, &cards);
    chain.write().start();
    let generation = animation.peek().generation;
    assert!(chain.peek().active);

    events::dispatch(&dom, "click", restart);

    assert!(*state.restarting.peek());
    assert!(!state.waiting_on_input());
    assert!(state.pending_input.peek().is_none());
    assert!(state.cards.peek().is_none());
    assert!(view.selected_card.peek().is_none());
    assert!(view.open_pile.peek().is_none());
    assert!(view.effects_to_select_from.peek().is_empty());
    assert!(!animation.peek().is_active());
    assert!(animation.peek().moving.is_empty());
    assert!(animation.peek().source.is_none());
    assert!(!chain.peek().active);
    assert!(chain.peek().displayed.is_empty());
    assert!(!chain.peek().has_pending_frames());
    animation
        .write()
        .advance(generation, AnimationPhase::Moving);
    chain.write().start();
    assert!(!animation.peek().is_active());
    assert!(!chain.peek().active);
    assert!(state.interaction_blocked(Some(animation)));
    STATE.with(|slot| *slot.borrow_mut() = None);
    VIEW.with(|slot| *slot.borrow_mut() = None);
    ANIMATION.with(|slot| *slot.borrow_mut() = None);
    CHAIN.with(|slot| *slot.borrow_mut() = None);
}

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
        let mut view = use_context_provider(ViewState::default);
        let mut duel = use_context_provider(|| Signal::new(None::<Duel>));
        let mut tracking = state.cards;
        let initial = DuelSetup {
            duel: first.clone(),
            cards: CardTracker::from_decks(&[1, 2], &[]).unwrap(),
        };
        install_duel(&mut state, &mut view, &mut duel, Some(initial.clone()));
        let selected = tracking.peek().as_ref().unwrap().cards()[0].id;
        view.selected_card.set(Some(selected));
        view.open_pile
            .set(Some(crate::ocgcore::constants::CardLocation::ExtraDeck));
        view.effects_to_select_from.set(vec![CardData {
            card_code: 1,
            ..Default::default()
        }]);
        state
            .pending_input
            .set(Some(PendingInput::YesNo("Activate?".into())));
        state.restarting.set(true);
        state.failure.set(Some("test failure".into()));
        tracking.set(None);
        install_duel(&mut state, &mut view, &mut duel, Some(initial));
        assert!(state.waiting_on_input());
        assert!(tracking.read().is_none());
        assert_eq!(*view.selected_card.peek(), Some(selected));
        assert!(view.open_pile.peek().is_some());
        assert!(!view.effects_to_select_from.peek().is_empty());
        let replacement = DuelSetup {
            duel: second.clone(),
            cards: CardTracker::from_decks(&[3], &[]).unwrap(),
        };
        install_duel(&mut state, &mut view, &mut duel, Some(replacement));
        assert!(!state.waiting_on_input());
        assert!(!(state.restarting)());
        assert!((state.failure)().is_none());
        assert!(view.selected_card.peek().is_none());
        assert!(view.open_pile.peek().is_none());
        assert!(view.effects_to_select_from.peek().is_empty());
        assert!(duel.read().as_ref().unwrap().ptr_eq(&second));
        let guard = tracking.read();
        let Some(layer) = &*guard else {
            panic!("Restart did not initialize a fresh tracker");
        };
        assert_eq!(layer.cards().len(), 1);
        assert_eq!(layer.cards()[0].code, 3);
        drop(guard);
        install_duel(&mut state, &mut view, &mut duel, None);
        assert!(duel.read().is_none());
        assert!(tracking.read().is_none());
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
