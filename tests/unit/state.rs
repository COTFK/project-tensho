use super::*;
use std::cell::Cell;

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn card_layer_initializer_matches_native_decks_and_custom_hand() {
    use crate::ui::card_layer::CardLayer;
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
                .filter(|card| card.address.location == location)
                .map(|card| card.code)
                .collect::<Vec<_>>(),
            duel.get_cards(location)
                .iter()
                .map(|card| card.unwrap().card_code)
                .collect::<Vec<_>>()
        );
    }
    duel.start();
    for _ in 0..1000 {
        let status = duel.process();
        let messages = duel.parse_messages();
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
        assert_eq!(phases, [Phase::Draw, Phase::Standby, Phase::Main1]);
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
