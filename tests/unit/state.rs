use super::*;
use ocgcore_ffi::constants::MSG_TAG_SWAP;
use std::cell::Cell;

#[test]
fn pending_input_reset_closes_current_pile() {
    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        for location in [
            CardLocation::Graveyard,
            CardLocation::Banishment,
            CardLocation::ExtraDeck,
        ] {
            state.open_pile.set(Some(location));
            assert_eq!(*state.open_pile.peek(), Some(location));
            state.clear_pending_input();
            assert_eq!(*state.open_pile.peek(), None);
        }
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn selection_tracks_moves_and_invalidates_on_shuffle_or_failure() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{MoveMessageData, ShuffleCardsMessageData};
    use crate::ui::card_layer::CardAddress;

    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }

    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        let mut layer = CardLayer::default();
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
        let selected = state.selected_card.peek().unwrap();
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
        reconcile_selected_card(&mut state, tracking.peek().as_ref().unwrap());
        assert!(state.is_selected(field));
        assert_eq!(state.selected_card.peek().unwrap(), selected);
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
        reconcile_selected_card(&mut state, tracking.peek().as_ref().unwrap());
        assert!(state.selected_card.peek().is_none());
        select_card(hand);
        assert_ne!(state.selected_card.peek().unwrap(), selected);
        state.tracking_failed("test".into(), None);
        assert!(state.selected_card.peek().is_none());
        select_card(hand);
        assert!(state.selected_card.peek().is_none());
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
    use crate::ui::card_layer::CardAddress;

    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            sequence: 0,
        };
        let mut layer = CardLayer::default();
        let id = layer.insert(address, 42).unwrap();
        state.selected_card.set(Some(id));
        let empty = CardLayer::default();
        reconcile_selected_card(&mut state, &empty);
        assert!(state.selected_card.peek().is_none());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn card_layer_initializer_matches_native_decks_and_custom_hand() {
    use crate::ui::card_layer::{CardLayer, CardPlacement};
    use ocgcore_ffi::constants::{TYPE_FUSION, TYPE_MONSTER, TYPE_NORMAL};
    use ocgcore_ffi::types::OCG_CardData;

    fn card_data(code: u32) -> OCG_CardData {
        static EMPTY_SETCODES: [u16; 1] = [0];
        OCG_CardData {
            code,
            alias: 0,
            setcodes: EMPTY_SETCODES.as_ptr().cast_mut(),
            r#type: if code >= 100 {
                TYPE_MONSTER | TYPE_FUSION
            } else {
                TYPE_MONSTER | TYPE_NORMAL
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

    let core = futures::executor::block_on(OCGCore::load(card_data, Box::new(|_| None), |_, _| {}))
        .unwrap();
    let duel = core.create_duel(3).unwrap();
    let mut deck = [10; 40];
    deck[..6].copy_from_slice(&[11, 12, 11, 13, 14, 15]);
    pull_ids_to_front(&mut deck, &[11, 11, 14]);
    deck.reverse();
    let extra = [100, 101, 100];
    let mut layer = CardLayer::from_decks(&deck, &extra).unwrap();

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
    for code in extra {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::ExtraDeck,
            0,
            10,
        );
    }
    for location in [CardLocation::Deck, CardLocation::ExtraDeck] {
        assert_eq!(
            layer
                .cards()
                .iter()
                .filter(|card| matches!(card.placement, CardPlacement::Zone(address) if address.location == location))
                .map(|card| card.code)
                .collect::<Vec<_>>(),
            duel.query_cards(Player::Player, location).unwrap()
                .iter()
                .map(|card| card.unwrap().card_code)
                .collect::<Vec<_>>()
        );
    }
    duel.start();
    for _ in 0..1000 {
        let status = duel.process();
        let messages = duel.parse_messages();
        for message in &messages {
            layer.apply(message).unwrap();
        }
        refresh_data(&mut layer, &duel).unwrap();
        if messages
            .iter()
            .any(|message| matches!(message, CoreMessage::Draw(_)))
        {
            assert_eq!(
                duel.query_cards(Player::Player, CardLocation::Hand)
                    .unwrap()
                    .iter()
                    .map(|card| card.unwrap().card_code)
                    .collect::<Vec<_>>(),
                [11, 11, 14]
            );
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Duel did not draw the starting hand");
}

#[test]
fn processing_continues_after_draw_until_an_input_prompt() {
    let processed = Cell::new(0);
    let handled = Cell::new(0);

    process_until_input(
        || {
            processed.set(processed.get() + 1);
            OCG_DuelStatus::OCG_DUEL_STATUS_CONTINUE
        },
        || {
            handled.set(handled.get() + 1);
            handled.get() == 2
        },
    );

    assert_eq!(processed.get(), 2);
    assert_eq!(handled.get(), 2);
}

#[test]
fn processing_continues_after_automatic_response_and_stops_at_duel_end() {
    let processed = Cell::new(0);
    let handled = Cell::new(0);

    process_until_input(
        || {
            processed.set(processed.get() + 1);
            if processed.get() == 1 {
                OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING
            } else {
                OCG_DuelStatus::OCG_DUEL_STATUS_END
            }
        },
        || {
            handled.set(handled.get() + 1);
            false
        },
    );

    assert_eq!(processed.get(), 2);
    assert_eq!(handled.get(), 2);
}

#[test]
fn dispatches_final_messages_when_duel_ends() {
    let mut handled = false;
    process_until_input(
        || OCG_DuelStatus::OCG_DUEL_STATUS_END,
        || {
            handled = true;
            false
        },
    );
    assert!(handled);
}

#[test]
fn finished_duel_stops_game_loop_and_reset_clears_flag() {
    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
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
fn failure_and_restart_block_input_and_cancel_playback() {
    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        let mut playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
        playback.write().begin(vec![], CardLayer::default());
        let generation = playback.peek().generation;
        state.fail("layout failed".into(), Some(playback));
        assert_eq!(state.failure.peek().as_deref(), Some("layout failed"));
        assert!(!playback.peek().active);
        assert!(state.interaction_blocked(Some(playback)));
        playback.write().finish(generation);
        assert!(state.interaction_blocked(Some(playback)));

        state.reset();
        assert!(!state.interaction_blocked(Some(playback)));
        state.restarting.set(true);
        assert!(state.interaction_blocked(Some(playback)));
        state.reset();
        assert!(!state.interaction_blocked(Some(playback)));
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
        let mut state = use_context_provider(UIState::default);
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        };
        let mut layer = CardLayer::default();
        let id = layer.insert(address, 42).unwrap();
        state.cards.set(Some(layer));
        state.selected_card.set(Some(id));
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
        assert!(!state.apply_card_message(&message, None));
        assert!(state.cards.peek().is_none());
        assert!(state.selected_card.peek().is_none());
        assert!(
            state
                .failure
                .peek()
                .as_deref()
                .unwrap()
                .contains("Card tracking failed")
        );
        assert!(!state.apply_card_message(&CoreMessage::ReverseDeck, None));
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
        let mut state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel)));
        let _playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
        state.tracking_failed("injected divergence".into(), Some(_playback));
        state.waiting_on_input.set(true);
        run_game_loop();
        assert_eq!(state.failure.peek().as_deref(), Some("injected divergence"));
        send_response(Response::No);
        assert!(*state.waiting_on_input.peek());
        COMPLETED.with(|completed| completed.set(true));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(COMPLETED.with(|completed| completed.get()));
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
        let mut state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel)));
        let mut playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
        state.waiting_on_input.set(true);
        playback.write().begin(vec![], CardLayer::default());
        send_response(Response::No);
        assert!(*state.waiting_on_input.peek());
        let generation = playback.peek().generation;
        playback.write().finish(generation);
        send_response(Response::No);
        assert!(!*state.waiting_on_input.peek());
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        };
        let id = CardLayer::default().insert(address, 1).unwrap();
        state.selected_card.set(Some(id));
        send_response(Response::Yes);
        assert!(state.selected_card.peek().is_some());
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
        let mut state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel)));
        state
            .cards
            .set(Some(CardLayer::from_decks(&deck, &[]).unwrap()));
        let tracking = state.cards;
        let mut playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
        run_game_loop();
        assert!(playback.peek().active);
        assert_eq!(playback.peek().moving.len(), 5);
        let paused = tracking.peek().clone();
        run_game_loop();
        assert_eq!(*tracking.peek(), paused);
        let generation = playback.peek().generation;
        playback.write().finish(generation);
        for _ in 0..100 {
            run_game_loop();
            if playback.peek().active {
                let generation = playback.peek().generation;
                playback.write().finish(generation);
            } else if *state.waiting_on_input.peek() {
                assert!(tracking.peek().is_some());
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
        let mut state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel.clone())));
        state.cards.set(Some(CardLayer::default()));
        let tracking = state.cards;

        let mut phases = Vec::new();
        for _ in 0..1000 {
            let status = duel.process();
            let waiting = handle_core_message();
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
        assert!((state.waiting_on_input)());
        assert!(tracking.read().is_some());
        assert_eq!(phases, [Phase::Draw, Phase::Standby, Phase::Main1]);
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn failed_tracking_stops_processing_without_playback_context() {
    thread_local! {
        static COMPLETED: Cell<bool> = const { Cell::new(false) };
    }
    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        state.cards.set(Some(CardLayer::default()));
        state.tracking_failed("test failure".to_owned(), None);
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
    let mut layer = CardLayer::default();
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
    let mut layer = CardLayer::default();
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
    let mut layer = CardLayer::default();
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
    refresh_data(&mut layer, &duel).unwrap();
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
    let mut layer = CardLayer::default();
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
    let error = refresh_data(&mut layer, &duel).unwrap_err();
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
    let mut layer = CardLayer::default();
    let address = CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
    };
    layer.insert(address, 99).unwrap();
    refresh_data(&mut layer, &duel).unwrap();
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
    let mut layer = CardLayer::default();
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
        refresh_data(&mut layer, &duel).unwrap();
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
