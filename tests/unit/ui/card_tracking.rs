use super::*;

#[test]
fn ignores_nonstructural_messages_and_stops_after_failure() {
    let mut tracking = CardTracking::Active(CardLayer::default());
    tracking.apply(&CoreMessage::NewTurn);
    assert!(matches!(tracking, CardTracking::Active(_)));
    tracking.apply(&CoreMessage::Unsupported(MSG_TAG_SWAP as u8));
    let CardTracking::Failed(error) = &tracking else {
        panic!("Expected an explicit structural tracking failure");
    };
    assert!(error.contains("Unsupported structural core message"));
    let failed = tracking.clone();
    tracking.apply(&CoreMessage::ReverseDeck);
    assert_eq!(tracking, failed);
}

#[test]
fn applies_ordered_moves_once_and_stops_atomically_on_invalid_input() {
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
    let mut tracking = CardTracking::Active(CardLayer::default());
    tracking.apply(&movement(
        info(CardLocationKind::None, 0),
        info(CardLocationKind::Known(CardLocation::Hand), 0),
    ));
    let CardTracking::Active(layer) = &tracking else {
        panic!("{tracking:?}")
    };
    let id = layer
        .card_at(CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence: 0,
        })
        .unwrap()
        .id;
    tracking.apply(&movement(
        info(CardLocationKind::Known(CardLocation::Hand), 0),
        info(CardLocationKind::Known(CardLocation::Graveyard), 0),
    ));
    let CardTracking::Active(layer) = &tracking else {
        panic!("{tracking:?}")
    };
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
    tracking.apply(&movement(
        info(CardLocationKind::Known(CardLocation::Hand), 0),
        info(CardLocationKind::Known(CardLocation::Graveyard), 1),
    ));
    assert!(matches!(tracking, CardTracking::Failed(_)));
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
fn reconciles_public_graveyard_codes_without_revealing_hidden_deck_order() {
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
    let mut tracking = CardTracking::Active(layer);
    tracking.reconcile(&duel);
    let CardTracking::Active(layer) = tracking else {
        panic!("Reconciliation failed")
    };
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
fn detects_core_divergence_and_preserves_failure() {
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
    let mut tracking = CardTracking::Active(layer);
    tracking.reconcile(&duel);
    let CardTracking::Failed(error) = &tracking else {
        panic!("Expected count mismatch")
    };
    assert!(error.contains("card count differs"));
    let failed = tracking.clone();
    tracking.reconcile(&duel);
    assert_eq!(tracking, failed);
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
    let mut tracking = CardTracking::Active(layer);
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
            tracking.apply(&message);
            assert!(
                matches!(tracking, CardTracking::Active(_)),
                "{tracking:?}; {message:?}"
            );
        }
        tracking.reconcile(&duel);
        assert!(matches!(tracking, CardTracking::Active(_)), "{tracking:?}");
        if status == OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING && saw_summon {
            assert!(saw_move && saw_set);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Native tracking test did not reach a prompt");
}
