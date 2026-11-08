use super::*;
use crate::duel::cards::CardPlacement;
use crate::ocgcore::constants::Player;
use crate::ocgcore::messages::SelectTributeMessageData;
use crate::ui::playback::{Playback, use_playback};
use ocgcore_ffi::constants::MSG_TAG_SWAP;
use std::cell::Cell;

fn provide_state() -> UIState {
    use_playback();
    use_context_provider(UIState::default)
}

fn handle_messages(messages: &[CoreMessage]) -> bool {
    let mut state = consume_context::<UIState>();
    let mut view = consume_context::<ViewState>();
    let controller = consume_context::<Signal<Option<DuelController>>>()().unwrap();
    handle_core_messages(
        &mut state,
        &mut view,
        &controller,
        messages,
        consume_context::<Playback>(),
    )
}

#[test]
fn pending_input_and_view_resets_are_independent() {
    fn app() -> Element {
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        let mut cards = CardTracker::default();
        let id = cards
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Hand,
                    sequence: 0,
                },
                42,
            )
            .unwrap();
        let effect = CardData {
            card_code: 42,
            ..Default::default()
        };
        for location in [
            CardLocation::Graveyard,
            CardLocation::Banishment,
            CardLocation::ExtraDeck,
        ] {
            view.open_pile.set(Some(location));
            view.selected_card.set(Some(id));
            view.effects_to_select_from.set(vec![effect]);
            assert_eq!(*view.open_pile.peek(), Some(location));
            state
                .pending_input
                .set(Some(PendingInput::YesNo("Activate?".into())));
            state.clear_pending_input();
            assert!(state.pending_input.peek().is_none());
            assert!(!state.waiting_on_input());
            assert_eq!(*view.open_pile.peek(), Some(location));
            assert_eq!(*view.selected_card.peek(), Some(id));
            assert_eq!(*view.effects_to_select_from.peek(), vec![effect]);
            state
                .pending_input
                .set(Some(PendingInput::YesNo("Activate?".into())));
            view.reset();
            assert_eq!(*view.open_pile.peek(), None);
            assert!(view.selected_card.peek().is_none());
            assert!(view.effects_to_select_from.peek().is_empty());
            assert_eq!(
                *state.pending_input.peek(),
                Some(PendingInput::YesNo("Activate?".into()))
            );
            assert!(state.waiting_on_input());
        }
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn view_reset_preserves_tracking_and_shared_tribute_progress() {
    fn app() -> Element {
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        let mut cards = CardTracker::default();
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            sequence: 0,
        };
        let id = cards.insert(address, 42).unwrap();
        let candidate = CardData {
            card_code: 42,
            controller: address.controller,
            location: address.location,
            action_index: Some(0),
            ..Default::default()
        };
        let tribute = SelectTributeMessageData {
            player: 0,
            is_cancelable: true,
            min_select: 1,
            max_select: 1,
            candidates: vec![candidate],
        };
        state.cards.set(Some(cards.clone()));
        let input = PendingInput::Tribute {
            message: tribute,
            selected: vec![0],
        };
        state.pending_input.set(Some(input.clone()));
        state.current_phase.set(Some(Phase::Main1));
        view.selected_card.set(Some(id));
        view.open_pile.set(Some(CardLocation::Graveyard));
        view.effects_to_select_from.set(vec![candidate]);

        view.reset();

        assert!(view.selected_card.peek().is_none());
        assert!(view.open_pile.peek().is_none());
        assert!(view.effects_to_select_from.peek().is_empty());
        assert_eq!(*state.cards.peek(), Some(cards));
        assert_eq!(*state.pending_input.peek(), Some(input));
        assert!(state.waiting_on_input());
        assert_eq!(*state.current_phase.peek(), Some(Phase::Main1));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn selection_tracks_moves_and_invalidates_on_shuffle_or_failure() {
    use crate::duel::cards::CardAddress;
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{MoveMessageData, ShuffleCardsMessageData};

    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }

    fn app() -> Element {
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        let mut layer = CardTracker::default();
        let hand = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        };
        let field = CardAddress {
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            sequence: 0,
        };
        layer.insert(hand, 42).unwrap();
        layer
            .insert(
                CardAddress {
                    sequence: 1,
                    ..hand
                },
                43,
            )
            .unwrap();
        let mut tracking = state.cards;
        tracking.set(Some(layer));
        select_card(hand);
        let selected = view.selected_card.peek().unwrap();
        let loc = |address: CardAddress| CardLocationInfo {
            controller: Some(address.controller),
            location: CardLocationKind::Known(address.location),
            sequence: address.sequence,
            position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
        };
        let movement = |from, to| {
            CoreMessage::Move(MoveMessageData {
                card_code: 42,
                previous: loc(from),
                current: loc(to),
                reason: MoveReason::EFFECT,
            })
        };
        tracking
            .write()
            .as_mut()
            .unwrap()
            .apply(&movement(hand, field))
            .unwrap();
        view.reconcile_selection(tracking.peek().as_ref().unwrap());
        assert!(view.is_selected(field, tracking.peek().as_ref()));
        assert_eq!(view.selected_card.peek().unwrap(), selected);
        tracking
            .write()
            .as_mut()
            .unwrap()
            .apply(&movement(field, hand))
            .unwrap();
        tracking
            .write()
            .as_mut()
            .unwrap()
            .apply(&CoreMessage::ShuffleHand(ShuffleCardsMessageData {
                player: Player::Player,
                card_codes: vec![42, 43],
            }))
            .unwrap();
        view.reconcile_selection(tracking.peek().as_ref().unwrap());
        assert!(view.selected_card.peek().is_none());
        select_card(hand);
        assert_ne!(view.selected_card.peek().unwrap(), selected);
        state.tracking_failed(&mut view, "test".into(), consume_context::<Playback>());
        assert!(view.selected_card.peek().is_none());
        select_card(hand);
        assert!(view.selected_card.peek().is_none());
        COMPLETED.with(|completed| completed.set(true));
        rsx! {}
    }
    COMPLETED.with(|completed| completed.set(false));
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
fn selected_card_is_cleared_when_it_disappears() {
    use crate::duel::cards::CardAddress;

    fn app() -> Element {
        let mut view = use_context_provider(ViewState::default);
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            sequence: 0,
        };
        let mut layer = CardTracker::default();
        let id = layer.insert(address, 42).unwrap();
        view.selected_card.set(Some(id));
        let empty = CardTracker::default();
        view.reconcile_selection(&empty);
        assert!(view.selected_card.peek().is_none());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn finished_duel_stops_game_loop_and_reset_clears_flag() {
    fn app() -> Element {
        let mut state = provide_state();
        state.duel_finished.set(true);
        run_game_loop();
        assert!(*state.duel_finished.peek());
        state.reset();
        assert!(!*state.duel_finished.peek());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn startup_suppression_does_not_block_processing() {
    fn app() -> Element {
        let mut state = provide_state();
        let playback = consume_context::<Playback>();
        assert!(state.modals_suppressed(false, playback));
        assert!(!state.processing_blocked(playback));
        assert!(!state.interaction_blocked(playback));

        state
            .pending_input
            .set(Some(PendingInput::YesNo("Activate?".into())));
        assert!(state.processing_blocked(playback));
        assert!(state.modals_suppressed(false, playback));
        assert!(!state.modals_suppressed(true, playback));
        state.clear_pending_input();
        state.duel_finished.set(true);
        assert!(state.processing_blocked(playback));
        assert!(!state.modals_suppressed(true, playback));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn either_playback_blocks_processing_and_interaction() {
    use crate::duel::controller::{ChainEvent, ChainLink};

    fn app() -> Element {
        let state = provide_state();
        let mut playback = consume_context::<Playback>();
        playback
            .animation
            .write()
            .begin(vec![], CardTracker::default());
        assert!(state.processing_blocked(playback));
        assert!(state.interaction_blocked(playback));
        assert!(state.modals_suppressed(true, playback));
        run_game_loop();
        playback.cancel();
        assert!(!state.processing_blocked(playback));

        playback.chain.write().record(ChainEvent::Added(ChainLink {
            number: 1,
            card: None,
        }));
        playback.chain.write().start();
        assert!(state.processing_blocked(playback));
        assert!(state.interaction_blocked(playback));
        assert!(state.modals_suppressed(true, playback));
        run_game_loop();
        playback.cancel();
        assert!(!state.processing_blocked(playback));
        assert!(!state.modals_suppressed(true, playback));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn failure_and_restart_block_input_and_cancel_playback() {
    fn app() -> Element {
        let mut state = provide_state();
        let mut playback = consume_context::<Playback>().animation;
        playback.write().begin(vec![], CardTracker::default());
        let generation = playback.peek().generation;
        state.fail("layout failed".into(), consume_context::<Playback>());
        assert_eq!(state.failure.peek().as_deref(), Some("layout failed"));
        assert!(!playback.peek().is_active());
        assert!(state.interaction_blocked(consume_context::<Playback>()));
        playback.write().finish(generation);
        assert!(state.interaction_blocked(consume_context::<Playback>()));

        state.reset();
        assert!(!state.interaction_blocked(consume_context::<Playback>()));
        state.restarting.set(true);
        assert!(state.interaction_blocked(consume_context::<Playback>()));
        state.reset();
        assert!(!state.interaction_blocked(consume_context::<Playback>()));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn tracking_failure_discards_partial_state_and_stops_messages() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::MoveMessageData;
    use crate::ocgcore::messages::move_card::MoveReason;

    fn app() -> Element {
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        };
        let mut layer = CardTracker::default();
        let id = layer.insert(address, 42).unwrap();
        state.cards.set(Some(layer));
        view.selected_card.set(Some(id));
        let info = CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Known(CardLocation::Hand),
            sequence: 0,
            position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
        };
        let message = CoreMessage::Move(MoveMessageData {
            card_code: 42,
            previous: info,
            current: CardLocationInfo {
                controller: None,
                ..info
            },
            reason: MoveReason::EFFECT,
        });
        assert!(!state.apply_card_message(&mut view, &message, consume_context::<Playback>()));
        assert!(state.cards.peek().is_none());
        assert!(view.selected_card.peek().is_none());
        assert!(
            state
                .failure
                .peek()
                .as_deref()
                .unwrap()
                .contains("Card tracking failed")
        );
        assert!(!state.apply_card_message(
            &mut view,
            &CoreMessage::ReverseDeck,
            consume_context::<Playback>()
        ));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn failed_tracking_blocks_layered_actions_until_restart() {
    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }
    fn app() -> Element {
        let core = futures::executor::block_on(OCGCore::load(
            |_| panic!("Unexpected card lookup"),
            Box::new(|_| None),
            |_, _| {},
        ))
        .unwrap();
        let duel = core.create_duel(5).unwrap();
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        let _playback = consume_context::<Playback>().animation;
        state.tracking_failed(
            &mut view,
            "injected divergence".into(),
            consume_context::<Playback>(),
        );
        state
            .pending_input
            .set(Some(PendingInput::YesNo("Activate?".into())));
        run_game_loop();
        assert_eq!(state.failure.peek().as_deref(), Some("injected divergence"));
        send_response(Response::No);
        assert!(state.waiting_on_input());
        COMPLETED.with(|completed| completed.set(true));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn request_messages_replace_actions_and_selection_progress() {
    use crate::ocgcore::Zone;
    use crate::ocgcore::constants::BattlePosition;
    use crate::ocgcore::messages::{
        AnnounceNumberMessageData, IdleMessageData, SelectCardMessageData, SelectChainMessageData,
        SelectEffectYNMessageData, SelectOptionMessageData, SelectPlaceMessageData,
        SelectPositionMessageData, SelectUnselectMessageData, SortCardMessageData,
    };

    fn app() -> Element {
        let core = test_core();
        let duel = core.create_duel(0).unwrap();
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        state.cards.set(Some(CardTracker::default()));
        let card = CardData {
            card_code: 42,
            action_index: Some(7),
            ..Default::default()
        };
        let idle = IdleMessageData {
            normal_summons: vec![card],
            special_summons: vec![card],
            monster_sets: vec![card],
            spell_trap_sets: vec![card],
            activatable_effects: vec![card],
            ..Default::default()
        };
        let tribute = SelectTributeMessageData {
            player: 0,
            is_cancelable: true,
            min_select: 1,
            max_select: 2,
            candidates: vec![card],
        };
        let chain = SelectChainMessageData {
            player: 0,
            effects: vec![card],
        };
        let cards = SelectCardMessageData {
            min_select: 1,
            max_select: 1,
            cards: vec![card],
        };
        let zones = vec![Zone {
            location: CardLocation::MonsterZone,
            sequence: 3,
        }];
        let positions = vec![BattlePosition::FaceUpAttack, BattlePosition::FaceUpDefense];
        let unselect = SelectUnselectMessageData {
            playerid: 0,
            finishable: true,
            cancelable: true,
            min_count: 1,
            max_count: 2,
            select_cards: vec![card],
            unselect_cards: vec![],
        };
        let options = SelectOptionMessageData {
            player: 0,
            options: vec![],
        };
        let numbers = AnnounceNumberMessageData {
            player: 0,
            numbers: vec![2, 4],
        };
        let sort = SortCardMessageData {
            player: 0,
            cards: vec![card],
        };
        for (message, expected) in [
            (CoreMessage::Idle(idle.clone()), PendingInput::Idle(idle)),
            (
                CoreMessage::SelectTribute(tribute.clone()),
                PendingInput::Tribute {
                    message: tribute.clone(),
                    selected: vec![],
                },
            ),
            (
                CoreMessage::SelectChain(chain.clone()),
                PendingInput::Chain(chain),
            ),
            (
                CoreMessage::SelectEffectYN(SelectEffectYNMessageData(CardData {
                    action_index: None,
                    ..card
                })),
                PendingInput::EffectYesNo(CardData {
                    action_index: None,
                    ..card
                }),
            ),
            (
                CoreMessage::SelectCard(cards.clone()),
                PendingInput::Cards(cards),
            ),
            (
                CoreMessage::SelectPlace(SelectPlaceMessageData {
                    player: 0,
                    count: 1,
                    zones: zones.clone(),
                }),
                PendingInput::Place(zones),
            ),
            (
                CoreMessage::SelectPosition(SelectPositionMessageData {
                    player: 0,
                    card_code: 42,
                    positions: positions.clone(),
                }),
                PendingInput::Position(positions),
            ),
            (
                CoreMessage::SelectUnselectCard(unselect.clone()),
                PendingInput::Unselect(unselect),
            ),
            (
                CoreMessage::SelectOption(options.clone()),
                PendingInput::Options(options),
            ),
            (
                CoreMessage::AnnounceNumber(numbers.clone()),
                PendingInput::Numbers(numbers),
            ),
            (
                CoreMessage::SortCard(sort.clone()),
                PendingInput::Sort(sort),
            ),
        ] {
            state.pending_input.set(Some(PendingInput::Tribute {
                message: tribute.clone(),
                selected: vec![7],
            }));
            view.open_pile.set(Some(CardLocation::Graveyard));
            assert!(handle_messages(&[message.clone()]), "{message:?}");
            assert_eq!(*state.pending_input.peek(), Some(expected), "{message:?}");
            assert!(state.waiting_on_input());
            assert_eq!(*view.open_pile.peek(), Some(CardLocation::Graveyard));
            state.clear_pending_input();
            assert!(!state.waiting_on_input());
            assert!(state.pending_input.peek().is_none());
        }

        state
            .pending_input
            .set(Some(PendingInput::Idle(IdleMessageData::default())));
        view.effects_to_select_from.set(vec![card]);
        assert!(!handle_messages(&[CoreMessage::SelectChain(
            SelectChainMessageData {
                player: 0,
                effects: vec![]
            }
        )]));
        assert!(state.pending_input.peek().is_none());
        assert!(!state.waiting_on_input());
        assert!(view.open_pile.peek().is_none());
        assert!(view.effects_to_select_from.peek().is_empty());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn consuming_a_response_clears_input_and_view_but_preserves_duel_state() {
    fn app() -> Element {
        let core = test_core();
        let duel = core.create_duel(0).unwrap();
        duel.add_card(Player::Player, 10, Player::Player, CardLocation::Hand, 0, 0);
        let mut cards = CardTracker::default();
        let id = cards
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Hand,
                    sequence: 0,
                },
                10,
            )
            .unwrap();
        duel.start();
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        state.cards.set(Some(cards));
        run_game_loop();
        assert!(state.waiting_on_input());
        let summon = match state.pending_input.peek().as_ref().unwrap() {
            PendingInput::Idle(actions) => actions.normal_summons[0],
            input => panic!("Expected idle actions, got {input:?}"),
        };
        view.selected_card.set(Some(id));
        view.open_pile.set(Some(CardLocation::ExtraDeck));
        view.effects_to_select_from.set(vec![summon]);
        let cards = state.cards.peek().clone();
        let phase = *state.current_phase.peek();

        send_response(Response::NormalSummon {
            index: summon.action_index.unwrap(),
        });

        assert!(!state.waiting_on_input());
        assert!(state.pending_input.peek().is_none());
        assert!(view.selected_card.peek().is_none());
        assert!(view.open_pile.peek().is_none());
        assert!(view.effects_to_select_from.peek().is_empty());
        assert_eq!(*state.cards.peek(), cards);
        assert_eq!(*state.current_phase.peek(), phase);
        assert_eq!(phase, Some(Phase::Main1));
        assert!(state.failure.peek().is_none());
        assert!(!*state.duel_finished.peek());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn responses_are_blocked_during_playback_and_after_a_prompt_is_consumed() {
    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }
    fn app() -> Element {
        let core = futures::executor::block_on(OCGCore::load(
            |_| panic!("Unexpected card lookup"),
            Box::new(|_| None),
            |_, _| {},
        ))
        .unwrap();
        let duel = core.create_duel(5).unwrap();
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        let mut playback = consume_context::<Playback>().animation;
        state
            .pending_input
            .set(Some(PendingInput::YesNo("Activate?".into())));
        playback.write().begin(vec![], CardTracker::default());
        send_response(Response::No);
        assert!(state.waiting_on_input());
        let generation = playback.peek().generation;
        playback.write().finish(generation);
        send_response(Response::No);
        assert!(!state.waiting_on_input());
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        };
        let id = CardTracker::default().insert(address, 1).unwrap();
        view.selected_card.set(Some(id));
        send_response(Response::Yes);
        assert!(view.selected_card.peek().is_some());
        COMPLETED.with(|completed| completed.set(true));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn layered_loop_pauses_on_a_real_draw_and_resumes_to_the_prompt() {
    use ocgcore_ffi::constants::{TYPE_MONSTER, TYPE_NORMAL};
    use ocgcore_ffi::types::OCG_CardData;
    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }
    fn card_data(code: u32) -> OCG_CardData {
        static EMPTY_SETCODES: [u16; 1] = [0];
        OCG_CardData {
            code,
            alias: 0,
            setcodes: EMPTY_SETCODES.as_ptr().cast_mut(),
            r#type: TYPE_MONSTER | TYPE_NORMAL,
            level: 4,
            attribute: 1,
            race: 1,
            attack: 1000,
            defense: 1000,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
        }
    }
    fn app() -> Element {
        let core =
            futures::executor::block_on(OCGCore::load(card_data, Box::new(|_| None), |_, _| {}))
                .unwrap();
        let duel = core.create_duel(5).unwrap();
        let deck = [10; 40];
        for code in deck {
            duel.add_card(
                Player::Player,
                code,
                Player::Player,
                CardLocation::Deck,
                0,
                0,
            );
        }
        duel.start();
        let mut state = provide_state();
        use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        state
            .cards
            .set(Some(CardTracker::from_decks(&deck, &[]).unwrap()));
        let tracking = state.cards;
        let mut playback = consume_context::<Playback>().animation;
        assert!(state.modals_suppressed(false, consume_context::<Playback>()));
        assert!(!state.processing_blocked(consume_context::<Playback>()));
        run_game_loop();
        assert!(playback.peek().is_active());
        assert_eq!(playback.peek().moving.len(), 5);
        let paused = tracking.peek().clone();
        run_game_loop();
        assert_eq!(*tracking.peek(), paused);
        let generation = playback.peek().generation;
        playback.write().finish(generation);
        for _ in 0..100 {
            run_game_loop();
            if playback.peek().is_active() {
                let generation = playback.peek().generation;
                playback.write().finish(generation);
            } else if state.waiting_on_input() {
                assert!(tracking.peek().is_some());
                assert!(state.modals_suppressed(false, consume_context::<Playback>()));
                assert!(!state.modals_suppressed(true, consume_context::<Playback>()));
                COMPLETED.with(|completed| completed.set(true));
                return rsx! {};
            } else if *state.duel_finished.peek() {
                panic!("Duel ended before a prompt");
            }
        }
        panic!("Duel did not reach a prompt");
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn layered_loop_starts_gathering_for_a_real_hand_shuffle() {
    use ocgcore_ffi::constants::{TYPE_MONSTER, TYPE_NORMAL};
    use ocgcore_ffi::types::OCG_CardData;

    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }

    fn card_data(code: u32) -> OCG_CardData {
        static EMPTY_SETCODES: [u16; 1] = [0];
        OCG_CardData {
            code,
            alias: 0,
            setcodes: EMPTY_SETCODES.as_ptr().cast_mut(),
            r#type: TYPE_MONSTER | TYPE_NORMAL,
            level: 4,
            attribute: 1,
            race: 1,
            attack: 1000,
            defense: 1000,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
        }
    }

    fn app() -> Element {
        let core =
            futures::executor::block_on(OCGCore::load(card_data, Box::new(|_| None), |_, _| {}))
                .unwrap();
        let duel = core.create_duel(5).unwrap();
        let deck = [10; 40];
        for code in deck {
            duel.add_card(
                Player::Player,
                code,
                Player::Player,
                CardLocation::Deck,
                0,
                0,
            );
        }
        core.load_script(
            &duel,
            br#"
            local effect = Effect.GlobalEffect()
            effect:SetType(0x0802)
            effect:SetCode(1113)
            effect:SetOperation(function() Duel.ShuffleHand(0) end)
            Duel.RegisterEffect(effect, 0)
            "#,
            "hand-shuffle-test.lua",
        )
        .unwrap();
        duel.start();
        let mut state = provide_state();
        use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        state
            .cards
            .set(Some(CardTracker::from_decks(&deck, &[]).unwrap()));
        let mut playback = consume_context::<Playback>().animation;
        for _ in 0..100 {
            run_game_loop();
            if playback.peek().phase
                == crate::ui::card_animation::AnimationPhase::HandShuffleGathering
            {
                assert!(state.interaction_blocked(consume_context::<Playback>()));
                assert!(playback.peek().source.is_some());
                COMPLETED.with(|completed| completed.set(true));
                return rsx! {};
            }
            if playback.peek().is_active() {
                let generation = playback.peek().generation;
                playback.write().finish(generation);
            } else if state.waiting_on_input() || *state.duel_finished.peek() {
                break;
            }
        }
        panic!("Duel did not animate the hand shuffle");
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn batched_chain_messages_pause_the_loop_and_track_source_cards() {
    use crate::duel::controller::ChainLink;
    use ocgcore_ffi::constants::{POS_FACEUP_ATTACK, TYPE_MONSTER, TYPE_NORMAL};
    use ocgcore_ffi::types::OCG_CardData;

    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }

    fn card_data(code: u32) -> OCG_CardData {
        static EMPTY_SETCODES: [u16; 1] = [0];
        OCG_CardData {
            code,
            alias: 0,
            setcodes: EMPTY_SETCODES.as_ptr().cast_mut(),
            r#type: TYPE_MONSTER | TYPE_NORMAL,
            level: 4,
            attribute: 1,
            race: 1,
            attack: 1000,
            defense: 1000,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
        }
    }

    fn app() -> Element {
        let core =
            futures::executor::block_on(OCGCore::load(card_data, Box::new(|_| None), |_, _| {}))
                .unwrap();
        let duel = core.create_duel(0).unwrap();
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            sequence: 0,
        };
        duel.add_card(
            Player::Player,
            10,
            Player::Player,
            CardLocation::MonsterZone,
            0,
            POS_FACEUP_ATTACK,
        );
        let mut layer = CardTracker::default();
        let card = layer.insert(address, 10).unwrap();
        let mut state = provide_state();
        use_context_provider(ViewState::default);
        state.cards.set(Some(layer));
        use_context_provider(|| Signal::new(Some(DuelController::new(duel))));
        let mut chain = consume_context::<Playback>().chain;
        let bytes = [
            33, 0, 0, 0, 70, 10, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 73, 1, 1, 0, 0, 0, 74,
        ];
        let messages = crate::ocgcore::messages::parse_messages(&bytes).unwrap();
        assert!(!handle_messages(&messages));
        assert!(chain.peek().active);
        assert_eq!(
            chain.peek().displayed,
            [ChainLink {
                number: 1,
                card: Some(card)
            }]
        );
        assert!(state.interaction_blocked(consume_context::<Playback>()));
        chain.write().cancel();
        assert!(!state.interaction_blocked(consume_context::<Playback>()));
        COMPLETED.with(|completed| completed.set(true));
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn handles_real_duel_messages_until_input() {
    fn app() -> Element {
        let core = futures::executor::block_on(OCGCore::load(
            |_| panic!("Unexpected card lookup"),
            Box::new(|_| None),
            |_, _| {},
        ))
        .unwrap();
        let duel = core.create_duel(5).unwrap();
        duel.start();
        let mut state = provide_state();
        use_context_provider(ViewState::default);
        use_context_provider(|| Signal::new(Some(DuelController::new(duel.clone()))));
        state.cards.set(Some(CardTracker::default()));
        let tracking = state.cards;

        let mut phases = Vec::new();
        for _ in 0..1000 {
            let status = duel.process();
            let waiting = handle_messages(&duel.parse_messages());
            if let Some(phase) = (state.current_phase)() {
                if phases.last() != Some(&phase) {
                    phases.push(phase);
                }
            }
            if waiting {
                break;
            }
            assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
        }
        assert!(state.waiting_on_input());
        assert!(tracking.read().is_some());
        assert_eq!(phases, [Phase::Draw, Phase::Standby, Phase::Main1]);
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn failed_tracking_stops_processing_with_inactive_playback() {
    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }
    fn app() -> Element {
        let mut state = provide_state();
        let mut view = use_context_provider(ViewState::default);
        state.cards.set(Some(CardTracker::default()));
        state.tracking_failed(
            &mut view,
            "test failure".to_owned(),
            consume_context::<Playback>(),
        );
        assert!(state.cards.peek().is_none());
        assert_eq!(state.failure.peek().as_deref(), Some("test failure"));
        COMPLETED.with(|completed| completed.set(true));
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
}

#[test]
fn ignores_nonstructural_messages_and_rejects_unsupported_structure() {
    let mut layer = CardTracker::default();
    layer.apply(&CoreMessage::NewTurn).unwrap();
    let error = layer
        .apply(&CoreMessage::Unsupported(MSG_TAG_SWAP as u8))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Unsupported structural core message")
    );
}

#[test]
fn applies_ordered_moves_once_and_stops_on_invalid_input() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::MoveMessageData;
    use crate::ocgcore::messages::move_card::MoveReason;

    let info = |location, sequence| CardLocationInfo {
        controller: Some(Player::Player),
        location,
        sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    };
    let movement = |previous, current| {
        CoreMessage::Move(MoveMessageData {
            card_code: 10,
            previous,
            current,
            reason: MoveReason::EFFECT,
        })
    };
    let mut layer = CardTracker::default();
    layer
        .apply(&movement(
            info(CardLocationKind::None, 0),
            info(CardLocationKind::Known(CardLocation::Hand), 0),
        ))
        .unwrap();
    let id = layer
        .card_at(CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        })
        .unwrap()
        .id;
    layer
        .apply(&movement(
            info(CardLocationKind::Known(CardLocation::Hand), 0),
            info(CardLocationKind::Known(CardLocation::Graveyard), 0),
        ))
        .unwrap();
    assert_eq!(
        layer
            .card_at(CardAddress {
                controller: Player::Player,
                location: CardLocation::Graveyard,
                sequence: 0
            })
            .unwrap()
            .id,
        id
    );
    assert!(
        layer
            .apply(&movement(
                info(CardLocationKind::Known(CardLocation::Graveyard), 0),
                CardLocationInfo {
                    controller: None,
                    ..info(CardLocationKind::Known(CardLocation::Hand), 0)
                },
            ))
            .is_err()
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn test_core() -> crate::ocgcore::OCGCore {
    use ocgcore_ffi::constants::{TYPE_MONSTER, TYPE_NORMAL};
    use ocgcore_ffi::types::OCG_CardData;
    fn card_data(code: u32) -> OCG_CardData {
        static EMPTY_SETCODES: [u16; 1] = [0];
        OCG_CardData {
            code,
            alias: 0,
            setcodes: EMPTY_SETCODES.as_ptr().cast_mut(),
            r#type: TYPE_MONSTER | TYPE_NORMAL,
            level: 4,
            attribute: 1,
            race: 1,
            attack: 1000,
            defense: 1000,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
        }
    }
    futures::executor::block_on(crate::ocgcore::OCGCore::load(
        card_data,
        Box::new(|_| None),
        |_, _| {},
    ))
    .unwrap()
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn refreshes_public_graveyard_codes_without_revealing_hidden_deck_order() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    duel.add_card(Player::Player, 10, Player::Player, CardLocation::Deck, 0, 0);
    duel.add_card(
        Player::Player,
        20,
        Player::Player,
        CardLocation::Graveyard,
        0,
        0,
    );
    let mut layer = CardTracker::default();
    let deck = CardAddress {
        controller: Player::Player,
        location: CardLocation::Deck,
        sequence: 0,
    };
    let grave = CardAddress {
        controller: Player::Player,
        location: CardLocation::Graveyard,
        sequence: 0,
    };
    layer.insert(deck, 0).unwrap();
    let id = layer.insert(grave, 0).unwrap();
    DuelController::new(duel.clone())
        .refresh_data(&mut layer)
        .unwrap();
    assert_eq!(layer.card_at(deck).unwrap().code, 0);
    assert_eq!(
        (
            layer.card_at(grave).unwrap().id,
            layer.card_at(grave).unwrap().code
        ),
        (id, 20)
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn reports_missing_data_for_tracked_cards() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let mut layer = CardTracker::default();
    layer
        .insert(
            CardAddress {
                controller: Player::Player,
                location: CardLocation::Deck,
                sequence: 0,
            },
            1,
        )
        .unwrap();
    let error = DuelController::new(duel.clone())
        .refresh_data(&mut layer)
        .unwrap_err();
    assert!(error.to_string().contains("No queried card"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn refreshes_only_tracked_cards_and_queries_stats_only_on_field() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    for location in [CardLocation::Deck, CardLocation::MonsterZone] {
        duel.add_card(Player::Player, 10, Player::Player, location, 0, 0);
    }
    let mut layer = CardTracker::default();
    let address = CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
    };
    layer.insert(address, 99).unwrap();
    DuelController::new(duel.clone())
        .refresh_data(&mut layer)
        .unwrap();
    let field = layer.card_at(address).unwrap();
    assert_eq!(field.code, 99);
    assert_eq!(field.data.unwrap().card_code, 10);
    assert_eq!(field.data.unwrap().attack, Some(1000));
    assert_eq!(
        duel.query_cards(Player::Player, CardLocation::Deck)
            .unwrap()[0]
            .unwrap()
            .attack,
        None
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tracks_native_set_summon_creation_and_movement_batches() {
    use ocgcore_ffi::constants::{
        POS_FACEUP_ATTACK, TYPE_MONSTER, TYPE_NORMAL, TYPE_SPELL, TYPE_TOKEN,
    };
    use ocgcore_ffi::enums::OCG_DuelStatus;
    use ocgcore_ffi::types::OCG_CardData;

    fn card_data(code: u32) -> OCG_CardData {
        static EMPTY_SETCODES: [u16; 1] = [0];
        OCG_CardData {
            code,
            alias: 0,
            setcodes: EMPTY_SETCODES.as_ptr().cast_mut(),
            r#type: match code {
                100 => TYPE_SPELL,
                400 => TYPE_MONSTER | TYPE_NORMAL | TYPE_TOKEN,
                _ => TYPE_MONSTER | TYPE_NORMAL,
            },
            level: 4,
            attribute: 1,
            race: 1,
            attack: 1000,
            defense: 1000,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
        }
    }
    let core = futures::executor::block_on(crate::ocgcore::OCGCore::load(
        card_data,
        Box::new(|_| None),
        |_, _| {},
    ))
    .unwrap();
    let duel = core.create_duel(0).unwrap();
    let mut layer = CardTracker::default();
    for (player, location, codes) in [
        (Player::Player, CardLocation::Deck, vec![1; 12]),
        (Player::Opponent, CardLocation::Deck, vec![2; 12]),
        (Player::Player, CardLocation::Hand, vec![100, 100, 100]),
        (Player::Player, CardLocation::MonsterZone, vec![201]),
    ] {
        for (sequence, code) in codes.into_iter().enumerate() {
            duel.add_card(Player::Player, code, player, location, 0, POS_FACEUP_ATTACK);
            layer
                .insert(
                    CardAddress {
                        controller: player,
                        location,
                        sequence: sequence as u32,
                    },
                    code,
                )
                .unwrap();
        }
    }
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            Duel.SendtoGrave(Duel.GetFieldCard(0, 4, 0), 0x40)
            Duel.SSet(0, Duel.GetFieldGroup(0, 2, 0), 0, false)
            local token = Duel.CreateToken(0, 400)
            Duel.SpecialSummon(token, 0, 0, 0, false, false, 1)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "tracking-integration.lua",
    )
    .unwrap();
    duel.start();
    let mut saw_move = false;
    let mut saw_set = false;
    let mut saw_summon = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if let CoreMessage::SelectPlace(data) = &message {
                let zone = data.zones.first().unwrap();
                duel.set_response(crate::ocgcore::Response::Place {
                    controller: data.player,
                    location: zone.location as u8,
                    index: zone.sequence,
                });
            }
            saw_move |= matches!(message, CoreMessage::Move(_));
            saw_set |= matches!(message, CoreMessage::SetCard(_));
            saw_summon |= matches!(message, CoreMessage::SpecialSummoning(_));
            layer
                .apply(&message)
                .unwrap_or_else(|error| panic!("{message:?}: {error:#}"));
        }
        DuelController::new(duel.clone())
            .refresh_data(&mut layer)
            .unwrap();
        for location in [
            CardLocation::Deck,
            CardLocation::Hand,
            CardLocation::MonsterZone,
            CardLocation::SpellTrapZone,
            CardLocation::Graveyard,
            CardLocation::Banishment,
            CardLocation::ExtraDeck,
        ] {
            let queried = duel.query_cards(Player::Player, location).unwrap();
            for card in layer.cards() {
                if let CardPlacement::Zone(address) = card.placement {
                    if address.controller == Player::Player && address.location == location {
                        assert_eq!(card.data, queried[address.sequence as usize]);
                    }
                }
            }
        }
        if status == OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING && saw_summon {
            assert!(saw_move && saw_set);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Native tracking test did not reach a prompt");
}
