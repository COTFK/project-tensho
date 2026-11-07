use super::*;
use std::cell::Cell;

#[test]
fn selection_tracks_moves_and_invalidates_on_shuffle_or_failure() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{MoveMessageData, ShuffleCardsMessageData};
    use crate::ui::card_layer::CardAddress;

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
        let mut tracking = use_context_provider(|| Signal::new(Some(CardTracking::Active(layer))));
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
            .apply(&movement(hand, field));
        reconcile_selected_card(&mut state, tracking.peek().as_ref().unwrap());
        assert!(state.selected_card.peek().unwrap().matches(field));
        assert_eq!(
            state.selected_card.peek().unwrap().card_id,
            selected.card_id
        );
        tracking
            .write()
            .as_mut()
            .unwrap()
            .apply(&movement(field, hand));
        tracking
            .write()
            .as_mut()
            .unwrap()
            .apply(&CoreMessage::ShuffleHand(ShuffleCardsMessageData {
                player: Player::Player,
                card_codes: vec![42],
            }));
        reconcile_selected_card(&mut state, tracking.peek().as_ref().unwrap());
        assert!(state.selected_card.peek().is_none());
        select_card(hand);
        assert_ne!(
            state.selected_card.peek().unwrap().card_id,
            selected.card_id
        );
        tracking.set(Some(CardTracking::Failed("test".into())));
        reconcile_selected_card(&mut state, tracking.peek().as_ref().unwrap());
        assert!(state.selected_card.peek().is_none());
        select_card(hand);
        assert!(state.selected_card.peek().unwrap().matches(hand));
        assert!(state.selected_card.peek().unwrap().card_id.is_none());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
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
        state.selected_card.set(Some(SelectedCard {
            location: address.location,
            index: 0,
            card_id: Some(id),
        }));
        let empty = CardTracking::Active(CardLayer::default());
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
    let layer = CardLayer::from_decks(&deck, &extra).unwrap();

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
            duel.get_cards(location)
                .iter()
                .map(|card| card.unwrap().card_code)
                .collect::<Vec<_>>()
        );
    }
    duel.start();
    let mut tracking = CardTracking::Active(layer);
    for _ in 0..1000 {
        let status = duel.process();
        let messages = duel.parse_messages();
        for message in &messages {
            tracking.apply(message);
        }
        tracking.reconcile(&duel);
        assert!(matches!(tracking, CardTracking::Active(_)), "{tracking:?}");
        if messages
            .iter()
            .any(|message| matches!(message, CoreMessage::Draw(_)))
        {
            assert_eq!(
                duel.get_raw_hand()
                    .iter()
                    .map(|card| card.code)
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
#[cfg(not(target_arch = "wasm32"))]
fn responses_are_blocked_during_playback_and_after_a_prompt_is_consumed() {
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
        let mut playback = use_context_provider(|| Signal::new(CardPlayback::default()));
        state.waiting_on_input.set(true);
        playback
            .write()
            .begin(vec![], CardLayer::default(), Default::default());
        send_response(Response::No);
        assert!(*state.waiting_on_input.peek());
        let generation = playback.peek().generation;
        playback.write().finish(generation);
        send_response(Response::No);
        assert!(!*state.waiting_on_input.peek());
        state.selected_card.set(Some(SelectedCard {
            location: CardLocation::Hand,
            index: 0,
            card_id: None,
        }));
        send_response(Response::Yes);
        assert!(state.selected_card.peek().is_some());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn layered_loop_pauses_on_a_real_draw_and_resumes_to_the_prompt() {
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
        let state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel)));
        let tracking = use_context_provider(|| {
            Signal::new(Some(CardTracking::Active(
                CardLayer::from_decks(&deck, &[]).unwrap(),
            )))
        });
        let mut playback = use_context_provider(|| {
            Signal::new(CardPlayback {
                enabled: true,
                ..Default::default()
            })
        });
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
                assert!(matches!(&*tracking.peek(), Some(CardTracking::Active(_))));
                return rsx! {};
            } else if playback.peek().finished {
                panic!("Duel ended before a prompt");
            }
        }
        panic!("Duel did not reach a prompt");
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
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
        let state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel.clone())));
        let tracking =
            use_context_provider(|| Signal::new(Some(CardTracking::Active(CardLayer::default()))));

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
        assert!(matches!(&*tracking.read(), Some(CardTracking::Active(_))));
        assert_eq!(phases, [Phase::Draw, Phase::Standby, Phase::Main1]);
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn legacy_processing_continues_when_tracking_has_failed() {
    fn app() -> Element {
        let core = futures::executor::block_on(OCGCore::load(
            |_| panic!("Unexpected card lookup"),
            Box::new(|_| None),
            |_, _| {},
        ))
        .unwrap();
        let duel = core.create_duel(5).unwrap();
        duel.start();
        let state = use_context_provider(UIState::default);
        use_context_provider(|| Signal::new(Some(duel.clone())));
        let tracking = use_context_provider(|| {
            Signal::new(Some(CardTracking::Failed("test failure".to_owned())))
        });
        for _ in 0..1000 {
            let status = duel.process();
            if handle_core_message() {
                assert!((state.waiting_on_input)());
                assert!(
                    matches!(&*tracking.read(), Some(CardTracking::Failed(error)) if error == "test failure")
                );
                return rsx! {};
            }
            assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
        }
        panic!("Duel did not reach an input prompt");
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
