use super::*;
use crate::duel::cards::CardTracker;
use crate::duel::controller::{ChainEvent, ChainLink, resolve_chain_event};
use crate::duel::input::PendingInput;
use crate::ocgcore::DuelCardData;
use crate::ui::card_animation::CardAnimationState;
use crate::ui::chain::ChainPlayback;
use crate::ui::playback::use_playback;

#[path = "events.rs"]
mod events;

#[test]
fn startup_combines_layout_core_and_both_playbacks_and_stays_ready_on_resize() {
    use std::cell::RefCell;

    thread_local! {
        static CONTEXT: RefCell<Option<(DuelState, DuelPlayback, Signal<bool>, Signal<LayoutStatus>)>> =
            const { RefCell::new(None) };
    }

    fn app() -> Element {
        let mut state = use_context_provider(DuelState::default);
        let playback = use_playback();
        let mut ready = use_signal(|| false);
        let layout = use_signal(LayoutStatus::default);
        use_effect(move || apply_layout_status(&mut state, &mut ready, playback, layout()));
        CONTEXT.with(|slot| *slot.borrow_mut() = Some((state, playback, ready, layout)));
        rsx! {}
    }

    fn settle(dom: &mut VirtualDom) {
        dom.render_immediate_to_vec();
        dom.process_events();
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.process_events();
    let (mut state, mut playback, ready, mut layout) = CONTEXT.with(|slot| slot.borrow().unwrap());
    assert!(!*ready.peek());
    assert!(!state.processing_blocked(playback));
    layout.set(LayoutStatus::Ready);
    settle(&mut dom);
    assert!(!*ready.peek());

    state
        .pending_input
        .set(Some(PendingInput::Confirmation("Activate?".into())));
    playback
        .animation
        .write()
        .begin(vec![], CardTracker::default());
    playback.chain.write().record(ChainEvent::Added(ChainLink {
        number: 1,
        card: None,
    }));
    playback.chain.write().start();
    settle(&mut dom);
    assert!(!*ready.peek());
    playback.animation.write().cancel();
    settle(&mut dom);
    assert!(!*ready.peek());
    playback.chain.write().cancel();
    settle(&mut dom);
    assert!(*ready.peek());

    layout.set(LayoutStatus::Pending);
    state.clear_pending_input();
    playback
        .animation
        .write()
        .begin(vec![], CardTracker::default());
    playback.chain.write().record(ChainEvent::Added(ChainLink {
        number: 2,
        card: None,
    }));
    playback.chain.write().start();
    settle(&mut dom);
    assert!(*ready.peek());
    assert!(state.failure.peek().is_none());
    layout.set(LayoutStatus::Failed("resize failed".into()));
    settle(&mut dom);
    assert!(*ready.peek());
    assert_eq!(state.failure.peek().as_deref(), Some("resize failed"));
    assert!(!playback.is_active());
    assert!(!playback.chain.peek().has_pending_frames());
    CONTEXT.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn a_finished_duel_can_complete_startup_after_layout_and_playback_settle() {
    fn app() -> Element {
        let mut state = use_context_provider(DuelState::default);
        let mut playback = use_playback();
        let mut ready = use_signal(|| false);
        state.duel_finished.set(true);
        apply_layout_status(&mut state, &mut ready, playback, LayoutStatus::Pending);
        assert!(!*ready.peek());
        playback
            .animation
            .write()
            .begin(vec![], CardTracker::default());
        apply_layout_status(&mut state, &mut ready, playback, LayoutStatus::Ready);
        assert!(!*ready.peek());
        playback.cancel();
        apply_layout_status(&mut state, &mut ready, playback, LayoutStatus::Ready);
        assert!(*ready.peek());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn layout_reports_cannot_unlock_failed_or_restarting_duels() {
    fn app() -> Element {
        let mut state = use_context_provider(DuelState::default);
        let playback = use_playback();
        let mut ready = use_signal(|| false);
        state.restarting.set(true);
        apply_layout_status(&mut state, &mut ready, playback, LayoutStatus::Ready);
        assert!(!*ready.peek());
        apply_layout_status(
            &mut state,
            &mut ready,
            playback,
            LayoutStatus::Failed("ignored".into()),
        );
        assert!(state.failure.peek().is_none());

        state.restarting.set(false);
        state.failure.set(Some("existing failure".into()));
        apply_layout_status(&mut state, &mut ready, playback, LayoutStatus::Ready);
        assert!(!*ready.peek());
        apply_layout_status(
            &mut state,
            &mut ready,
            playback,
            LayoutStatus::Failed("ignored".into()),
        );
        assert_eq!(state.failure.peek().as_deref(), Some("existing failure"));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn restart_button_cancels_playback_and_clears_pending_input_and_view() {
    use crate::duel::cards::CardAddress;
    use crate::ocgcore::constants::{CardLocation, Player};
    use crate::ocgcore::messages::CoreMessage;
    use crate::ui::card_animation::CardAnimationPhase;
    use dioxus::core::{AttributeValue, Mutation};
    use std::cell::RefCell;

    thread_local! {
        static STATE: RefCell<Option<DuelState>> = const { RefCell::new(None) };
        static VIEW: RefCell<Option<DuelViewState>> = const { RefCell::new(None) };
        static ANIMATION: RefCell<Option<Signal<CardAnimationState>>> = const { RefCell::new(None) };
        static CHAIN: RefCell<Option<Signal<ChainPlayback>>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let playback = use_hook(DuelPlayback::default);
        let view = use_hook(DuelViewState::default);
        let state = use_hook(|| {
            let mut state = DuelState::default();
            state.restarting.set(true);
            state
        });
        let controller = use_signal(|| None::<DuelController>);
        let controller_input = use_hook(move || ReadSignal::new(controller));
        use_duel_contexts(state, view, playback, controller_input);
        let duel_resource = use_resource(std::future::pending::<anyhow::Result<DuelSetup>>);
        let screen = DuelScreen(DuelScreenProps {
            state,
            view,
            playback,
            controller: controller_input,
            duel_resource,
            deck_codes: vec![],
        });
        assert!(try_consume_context::<DuelState>().is_none());
        assert!(try_consume_context::<DuelViewState>().is_none());
        assert!(try_consume_context::<DuelPlayback>().is_none());
        STATE.with(|slot| *slot.borrow_mut() = Some(state));
        VIEW.with(|slot| *slot.borrow_mut() = Some(view));
        ANIMATION.with(|slot| *slot.borrow_mut() = Some(playback.animation));
        CHAIN.with(|slot| *slot.borrow_mut() = Some(playback.chain));
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
                controller: Player::Local,
                location: CardLocation::MonsterZone,
                sequence: 0,
            },
            1,
        )
        .unwrap();
    state.cards.set(Some(cards.clone()));
    view.selected_card.set(Some(card));
    view.open_pile.set(Some(CardLocation::Graveyard));
    view.effects_to_select_from.set(vec![DuelCardData {
        card_code: 1,
        ..Default::default()
    }]);
    state
        .pending_input
        .set(Some(PendingInput::Confirmation("Activate?".into())));
    animation.write().begin(vec![card], cards.clone());
    let message = CoreMessage::try_from(
        [
            33, 0, 0, 0, 70, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0,
        ]
        .as_slice(),
    )
    .unwrap();
    chain
        .write()
        .record(resolve_chain_event(&message, &cards).unwrap());
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
        .advance(generation, CardAnimationPhase::Moving);
    chain.write().start();
    assert!(!animation.peek().is_active());
    assert!(!chain.peek().active);
    assert!(state.interaction_blocked(DuelPlayback { animation, chain }));
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
        rsx! { InteractionGate { blocked: active(), button { "Action" } } }
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
fn bound_actions_responses_and_right_click_need_only_explicit_inputs() {
    use crate::duel::cards::CardAddress;
    use crate::duel::input::CardIntent;
    use crate::ocgcore::Response;
    use crate::ocgcore::constants::{CardLocation, Player};
    use crate::ocgcore::messages::IdleMessageData;
    use dioxus::core::Mutation;
    use std::cell::RefCell;

    thread_local! {
        static INPUTS: RefCell<Option<(DuelState, DuelViewState, DuelPlayback)>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let mut state = use_hook(DuelState::default);
        let view = use_hook(DuelViewState::default);
        let playback = use_hook(DuelPlayback::default);
        let controller = use_signal(|| {
            let core = futures::executor::block_on(OCGCore::load(
                |_| panic!("Unexpected card lookup"),
                Box::new(|_| None),
                |_, _| {},
            ))
            .unwrap();
            Some(DuelController::new(core.create_duel(0).unwrap()))
        });
        let hand = CardAddress {
            controller: Player::Local,
            location: CardLocation::Hand,
            sequence: 0,
        };
        use_hook(move || {
            let mut cards = CardTracker::default();
            cards.insert(hand, 42).unwrap();
            state.cards.set(Some(cards));
        });
        let controller_input = use_hook(move || ReadSignal::new(controller));
        use_duel_contexts(state, view, playback, controller_input);
        assert!(try_consume_context::<DuelState>().is_none());
        assert!(try_consume_context::<DuelViewState>().is_none());
        assert!(try_consume_context::<DuelPlayback>().is_none());
        assert!(try_consume_context::<Signal<Option<DuelController>>>().is_none());
        let onaction = use_context::<EventHandler<CardIntent>>();
        let onresponse = use_context::<EventHandler<Response>>();
        INPUTS.with(|slot| *slot.borrow_mut() = Some((state, view, playback)));
        rsx! {
            main {
                id: "coordination",
                oncontextmenu: move |event| handle_right_click(state, view, playback, controller_input, event),
                button { id: "select", onclick: move |_| onaction.call(CardIntent::Select(hand)), "Select" }
                button { id: "respond", onclick: move |_| onresponse.call(Response::No), "Respond" }
            }
        }
    }

    events::install();
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let clicks = initial
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "click" => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(clicks.len(), 2);
    let select = clicks[0];
    let respond = clicks[1];
    let main = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "contextmenu" => Some(*id),
            _ => None,
        })
        .expect("Right-click handler should be mounted");
    dom.process_events();
    let (mut state, mut view, mut playback) = INPUTS.with(|slot| slot.borrow().unwrap());
    events::dispatch(&dom, "click", select);
    dom.render_immediate_to_vec();
    let selected = *view.selected_card.peek();
    assert!(selected.is_some());
    view.open_pile.set(Some(CardLocation::Graveyard));
    events::dispatch(&dom, "click", respond);
    assert_eq!(*view.selected_card.peek(), selected);
    assert_eq!(*view.open_pile.peek(), Some(CardLocation::Graveyard));
    state
        .pending_input
        .set(Some(PendingInput::Confirmation("Activate?".into())));
    events::dispatch(&dom, "click", respond);
    assert!(!state.waiting_on_input());
    assert!(view.selected_card.peek().is_none());
    assert!(view.open_pile.peek().is_none());
    state.pending_input.set(Some(PendingInput::ActionSelection(
        IdleMessageData::default(),
    )));
    events::dispatch(&dom, "click", select);
    view.open_pile.set(Some(CardLocation::ExtraDeck));
    events::dispatch(&dom, "contextmenu", main);
    assert!(state.waiting_on_input());
    assert_eq!(*view.selected_card.peek(), selected);
    assert!(view.open_pile.peek().is_none());
    state
        .pending_input
        .set(Some(PendingInput::Confirmation("Activate?".into())));
    view.open_pile.set(Some(CardLocation::Graveyard));
    view.effects_to_select_from
        .set(vec![DuelCardData::default()]);
    playback
        .animation
        .write()
        .begin(vec![], CardTracker::default());
    playback.chain.write().record(ChainEvent::Added(ChainLink {
        number: 1,
        card: None,
    }));
    playback.chain.write().start();
    for animation_active in [true, false] {
        if !animation_active {
            playback.animation.write().cancel();
        }
        events::dispatch(&dom, "contextmenu", main);
        events::dispatch(&dom, "click", respond);
        assert!(state.waiting_on_input());
        assert_eq!(*view.selected_card.peek(), selected);
        assert_eq!(*view.open_pile.peek(), Some(CardLocation::Graveyard));
        assert!(!view.effects_to_select_from.peek().is_empty());
    }
    playback.chain.write().cancel();
    events::dispatch(&dom, "contextmenu", main);
    assert!(!state.waiting_on_input());
    assert!(view.selected_card.peek().is_none());
    assert!(view.open_pile.peek().is_none());
    assert!(view.effects_to_select_from.peek().is_empty());
    INPUTS.with(|slot| *slot.borrow_mut() = None);
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
        let first = DuelController::new(core.create_duel(0).unwrap());
        let second = DuelController::new(core.create_duel(0).unwrap());
        let mut state = use_context_provider(DuelState::default);
        let playback = use_playback();
        let mut animation = playback.animation;
        let mut chain = playback.chain;
        let mut view = use_context_provider(DuelViewState::default);
        let mut duel = use_context_provider(|| Signal::new(None::<DuelController>));
        let mut tracking = state.cards;
        let initial = DuelSetup {
            controller: first.clone(),
            cards: CardTracker::from_decks(&[1, 2], &[]).unwrap(),
        };
        install_duel(
            &mut state,
            &mut view,
            playback,
            &mut duel,
            Some(initial.clone()),
        );
        let selected = tracking.peek().as_ref().unwrap().cards()[0].id;
        animation
            .write()
            .begin(vec![selected], tracking.peek().as_ref().unwrap().clone());
        let generation = animation.peek().generation;
        for number in [1, 2] {
            chain.write().record(ChainEvent::Added(ChainLink {
                number,
                card: Some(selected),
            }));
        }
        chain.write().start();
        view.selected_card.set(Some(selected));
        view.open_pile
            .set(Some(crate::ocgcore::constants::CardLocation::ExtraDeck));
        view.effects_to_select_from.set(vec![DuelCardData {
            card_code: 1,
            ..Default::default()
        }]);
        state
            .pending_input
            .set(Some(PendingInput::Confirmation("Activate?".into())));
        state.restarting.set(true);
        state.failure.set(Some("test failure".into()));
        tracking.set(None);
        install_duel(&mut state, &mut view, playback, &mut duel, Some(initial));
        assert!(state.waiting_on_input());
        assert!(tracking.read().is_none());
        assert_eq!(*view.selected_card.peek(), Some(selected));
        assert!(view.open_pile.peek().is_some());
        assert!(!view.effects_to_select_from.peek().is_empty());
        assert!(animation.peek().is_active());
        assert_eq!(animation.peek().generation, generation);
        assert!(chain.peek().active);
        assert!(chain.peek().has_pending_frames());
        let replacement = DuelSetup {
            controller: second.clone(),
            cards: CardTracker::from_decks(&[3], &[]).unwrap(),
        };
        install_duel(
            &mut state,
            &mut view,
            playback,
            &mut duel,
            Some(replacement),
        );
        assert!(!state.waiting_on_input());
        assert!(!(state.restarting)());
        assert!((state.failure)().is_none());
        assert!(view.selected_card.peek().is_none());
        assert!(view.open_pile.peek().is_none());
        assert!(view.effects_to_select_from.peek().is_empty());
        assert!(!playback.is_active());
        assert!(animation.peek().source.is_none());
        assert!(animation.peek().moving.is_empty());
        assert_ne!(animation.peek().generation, generation);
        assert!(chain.peek().displayed.is_empty());
        assert!(!chain.peek().has_pending_frames());
        assert!(duel.read().as_ref().unwrap().ptr_eq(&second));
        let guard = tracking.read();
        let Some(layer) = &*guard else {
            panic!("Restart did not initialize a fresh tracker");
        };
        assert_eq!(layer.cards().len(), 1);
        assert_eq!(layer.cards()[0].code, 3);
        let card = layer.cards()[0].id;
        let source = layer.clone();
        drop(guard);
        animation.write().begin(vec![card], source);
        animation.write().advance(
            generation,
            crate::ui::card_animation::CardAnimationPhase::Moving,
        );
        assert!(animation.peek().is_active());
        chain.write().record(ChainEvent::Added(ChainLink {
            number: 3,
            card: Some(card),
        }));
        chain.write().start();
        install_duel(&mut state, &mut view, playback, &mut duel, None);
        assert!(duel.read().is_none());
        assert!(tracking.read().is_none());
        assert!(!playback.is_active());
        assert!(chain.peek().displayed.is_empty());
        assert!(!chain.peek().has_pending_frames());
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
