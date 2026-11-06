use super::*;
use crate::ocgcore::CardType;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Phase, Player, WinReason, Winner};
use crate::ocgcore::location_info::{CardLocationKind, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, Hint, HintMessageData, WinMessageData};
use crate::ui::card_layer::{CardAddress, CardLayer, CardPlacement};
use ocgcore_ffi::constants::{POS_FACEUP_ATTACK, TYPE_MONSTER, TYPE_NORMAL};
use ocgcore_ffi::enums::OCG_DuelStatus;
use ocgcore_ffi::types::{OCG_CardData, OCG_NewCardInfo};

fn test_card_data(code: u32) -> OCG_CardData {
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

fn test_core() -> OCGCore {
    futures::executor::block_on(OCGCore::load(
        test_card_data,
        Box::new(|_| None),
        super::super::callbacks::log_core_message,
    ))
    .unwrap()
}

fn startup_messages(player_cards: u32, opponent_cards: u32) -> Vec<CoreMessage> {
    let core = test_core();
    let duel = core.create_duel(5).unwrap();

    for (team, count) in [(0, player_cards), (1, opponent_cards)] {
        for code in 1..=count {
            let card = OCG_NewCardInfo {
                team,
                duelist: 0,
                code,
                con: team,
                loc: CardLocation::Deck as u32,
                seq: 0,
                pos: 0,
            };
            unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
        }
    }

    duel.start();
    let mut messages = Vec::new();
    for _ in 0..1000 {
        let status = duel.process();
        messages.extend(duel.parse_messages());
        if status != OCG_DuelStatus::OCG_DUEL_STATUS_CONTINUE {
            assert_eq!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING);
            return messages;
        }
    }
    panic!("Duel did not reach an input prompt or end");
}

#[test]
fn queries_native_core_cards_and_empty_zones() {
    let core = test_core();
    let duel = core.create_duel(5).unwrap();
    for (code, sequence) in [(101, 1), (102, 4)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::MonsterZone,
            sequence,
            POS_FACEUP_ATTACK,
        );
    }
    duel.add_card(
        Player::Player,
        103,
        Player::Player,
        CardLocation::Hand,
        0,
        POS_FACEUP_ATTACK,
    );

    let monsters = duel.get_cards(CardLocation::MonsterZone);
    assert_eq!(monsters.len(), 7);
    for index in [0, 2, 3, 5, 6] {
        assert_eq!(monsters[index], None);
    }
    for (index, code) in [(1, 101), (4, 102)] {
        let card = monsters[index].unwrap();
        assert_eq!(card.card_code, code);
        assert_eq!(card.controller, Player::Player);
        assert_eq!(card.location, CardLocation::MonsterZone);
        assert_eq!(card.sequence, index as u8);
        assert_eq!(card.position, Some(BattlePosition::FaceUpAttack));
        assert_eq!(card.card_type, CardType::MONSTER | CardType::NORMAL);
        assert_eq!(card.level, Some(4));
        assert_eq!(card.attack, Some(1000));
        assert_eq!(card.defense, Some(1000));
    }
    assert_eq!(duel.get_cards(CardLocation::Graveyard), vec![]);
    let hand = duel.get_raw_hand();
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].index, 0);
    assert_eq!(hand[0].code, 103);
}

#[test]
fn sandbox_continues_after_either_player_decks_out() {
    for (player_cards, opponent_cards, winner) in [
        (5, 0, Winner::Player(Player::Player)),
        (0, 5, Winner::Player(Player::Opponent)),
    ] {
        let messages = startup_messages(player_cards, opponent_cards);
        let win_index = messages
            .iter()
            .position(|message| {
                *message
                    == CoreMessage::Win(WinMessageData {
                        winner,
                        reason: WinReason::DeckOut,
                    })
            })
            .expect("Expected a deck-out win");
        assert!(
            messages[win_index + 1..]
                .iter()
                .any(|message| matches!(message, CoreMessage::Idle(_)))
        );
    }
}

#[test]
fn startup_messages_include_phase_changes_before_idle() {
    let messages = startup_messages(10, 10);
    let phases: Vec<_> = messages
        .iter()
        .filter_map(|message| match message {
            CoreMessage::NewPhase(data) => Some(data.phase),
            _ => None,
        })
        .collect();
    assert_eq!(phases, [Phase::Draw, Phase::Standby, Phase::Main1]);
    assert!(messages.contains(&CoreMessage::Hint(HintMessageData {
        player: Player::Player,
        hint: Hint::Event(27),
    })));
    assert!(matches!(messages.last(), Some(CoreMessage::Idle(_))));
}

#[test]
fn normal_summon_emits_move_summoning_and_summoned() {
    let core = test_core();
    let duel = core.create_duel(5).unwrap();
    let mut layer = CardLayer::default();
    for team in 0..=1 {
        for code in 1..=10 {
            let card = OCG_NewCardInfo {
                team,
                duelist: 0,
                code,
                con: team,
                loc: CardLocation::Deck as u32,
                seq: 0,
                pos: 0,
            };
            unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
            layer
                .insert(
                    CardAddress {
                        controller: Player::try_from(team).unwrap(),
                        location: CardLocation::Deck,
                        sequence: code - 1,
                    },
                    code,
                )
                .unwrap();
        }
    }
    let initial_deck: Vec<_> = (0..10)
        .map(|sequence| {
            layer
                .card_at(CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Deck,
                    sequence,
                })
                .unwrap()
                .id
        })
        .collect();
    duel.start();
    let mut summoned_code = None;
    let mut moved_location = None;
    let mut summoning_location = None;
    let mut summoned = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if let CoreMessage::Move(data) = &message {
                let source_id = if Some(data.card_code) == summoned_code {
                    Some(
                        layer
                            .card_at(CardAddress {
                                controller: Player::Player,
                                location: CardLocation::Hand,
                                sequence: data.previous.sequence,
                            })
                            .unwrap()
                            .id,
                    )
                } else {
                    None
                };
                layer.apply(&message).unwrap();
                if let Some(id) = source_id {
                    assert_eq!(
                        layer
                            .card_at(CardAddress {
                                controller: Player::Player,
                                location: CardLocation::MonsterZone,
                                sequence: data.current.sequence,
                            })
                            .unwrap()
                            .id,
                        id
                    );
                }
            } else if matches!(message, CoreMessage::Draw(_)) {
                layer.apply(&message).unwrap();
            }
            match message {
                CoreMessage::Idle(data) if summoned_code.is_none() => {
                    for location in [CardLocation::Deck, CardLocation::Hand] {
                        for (sequence, core_card) in duel.get_cards(location).iter().enumerate() {
                            let tracked = layer
                                .card_at(CardAddress {
                                    controller: Player::Player,
                                    location,
                                    sequence: sequence as u32,
                                })
                                .unwrap();
                            assert_eq!(tracked.code, core_card.unwrap().card_code);
                            if location == CardLocation::Hand {
                                assert_eq!(tracked.id, initial_deck[9 - sequence]);
                            }
                        }
                        assert_eq!(
                            layer
                                .cards()
                                .iter()
                                .filter(|card| matches!(card.placement, CardPlacement::Zone(address)
                                    if address.controller == Player::Player && address.location == location))
                                .count(),
                            duel.count_location(Player::Player, location) as usize
                        );
                    }
                    let card = data.normal_summons.first().expect("No summonable card");
                    summoned_code = Some(card.card_code);
                    duel.set_response(Response::NormalSummon {
                        index: card.action_index.expect("No summon action index"),
                    });
                }
                CoreMessage::SelectPlace(data) => {
                    let zone = data.zones.first().expect("No available zone");
                    duel.set_response(Response::Place {
                        controller: data.player,
                        location: zone.location as u8,
                        index: zone.sequence,
                    });
                }
                CoreMessage::Move(data) if Some(data.card_code) == summoned_code => {
                    assert_eq!(
                        data.previous.location,
                        CardLocationKind::Known(CardLocation::Hand)
                    );
                    assert_eq!(
                        data.current.location,
                        CardLocationKind::Known(CardLocation::MonsterZone)
                    );
                    moved_location = Some(data.current);
                }
                CoreMessage::Summoning(data) if Some(data.card_code) == summoned_code => {
                    summoning_location = Some(data.location);
                }
                CoreMessage::Summoned => {
                    assert!(summoning_location.is_some());
                    summoned = true;
                }
                _ => {}
            }
        }
        if let (Some(moved), Some(summoning), true) = (moved_location, summoning_location, summoned)
        {
            assert_eq!(summoning, moved);
            let tracked = layer
                .card_at(CardAddress {
                    controller: Player::Player,
                    location: CardLocation::MonsterZone,
                    sequence: moved.sequence,
                })
                .unwrap();
            let queried = duel.get_cards(CardLocation::MonsterZone);
            assert_eq!(
                tracked.code,
                queried[moved.sequence as usize].unwrap().card_code
            );
            for (sequence, core_card) in duel.get_cards(CardLocation::Hand).iter().enumerate() {
                assert_eq!(
                    layer
                        .card_at(CardAddress {
                            controller: Player::Player,
                            location: CardLocation::Hand,
                            sequence: sequence as u32,
                        })
                        .unwrap()
                        .code,
                    core_card.unwrap().card_code
                );
            }
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Summon did not emit move, summoning, and summoned messages");
}

#[test]
fn tracks_native_overlay_moves_and_host_removal() {
    let core = test_core();
    let duel = core.create_duel(5).unwrap();
    let mut layer = CardLayer::default();
    for (team, player) in [(0, Player::Player), (1, Player::Opponent)] {
        for code in 1..=10 {
            let card = OCG_NewCardInfo {
                team,
                duelist: 0,
                code,
                con: team,
                loc: CardLocation::Deck as u32,
                seq: 0,
                pos: 0,
            };
            unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
            layer
                .insert(
                    CardAddress {
                        controller: player,
                        location: CardLocation::Deck,
                        sequence: code - 1,
                    },
                    code,
                )
                .unwrap();
        }
    }
    let field = |sequence| CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence,
    };
    for (code, sequence) in [(100, 0), (101, 1), (102, 2)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::MonsterZone,
            sequence,
            POS_FACEUP_ATTACK,
        );
        layer.insert(field(sequence), code).unwrap();
    }
    let host = layer.card_at(field(0)).unwrap().id;
    let first = layer.card_at(field(1)).unwrap().id;
    let second = layer.card_at(field(2)).unwrap().id;
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local host = Duel.GetFieldCard(0, 4, 0)
            local first = Duel.GetFieldCard(0, 4, 1)
            local second = Duel.GetFieldCard(0, 4, 2)
            Duel.Overlay(host, first)
            Duel.Overlay(host, second)
            Duel.MoveSequence(host, 3)
            Duel.SendtoGrave(first, 0x40)
            Duel.SendtoGrave(host, 0x40)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "overlay-test.lua",
    )
    .unwrap();

    duel.start();
    let mut attached = 0;
    let mut detached = 0;
    let mut host_moved = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if matches!(message, CoreMessage::Draw(_) | CoreMessage::Move(_)) {
                layer.apply(&message).unwrap();
            }
            if let CoreMessage::Move(data) = message {
                match (data.previous.location, data.current.location) {
                    (_, CardLocationKind::Overlay { .. }) => {
                        attached += 1;
                        let expected = if data.card_code == 101 { first } else { second };
                        assert_eq!(layer.material_at(host, attached - 1).unwrap().id, expected);
                    }
                    (
                        CardLocationKind::Overlay { .. },
                        CardLocationKind::Known(CardLocation::Graveyard),
                    ) => {
                        detached += 1;
                        let expected = if data.card_code == 101 { first } else { second };
                        assert_eq!(
                            layer
                                .card_at(CardAddress {
                                    controller: Player::Player,
                                    location: CardLocation::Graveyard,
                                    sequence: data.current.sequence,
                                })
                                .unwrap()
                                .id,
                            expected
                        );
                    }
                    (
                        CardLocationKind::Known(CardLocation::MonsterZone),
                        CardLocationKind::Known(CardLocation::MonsterZone),
                    ) if data.card_code == 100 => {
                        assert_eq!(layer.card_at(field(3)).unwrap().id, host);
                        assert_eq!(layer.material_at(host, 0).unwrap().id, first);
                        assert_eq!(layer.material_at(host, 1).unwrap().id, second);
                        let query = ocgcore_ffi::types::OCG_QueryInfo {
                            flags: ocgcore_ffi::constants::QUERY_OVERLAY_CARD,
                            con: Player::Player as u8,
                            loc: CardLocation::MonsterZone as u32,
                            seq: 3,
                            overlay_seq: 0,
                        };
                        let mut length = 0;
                        let ptr = unsafe {
                            ocgcore_ffi::OCG_DuelQuery(duel.handle(), &mut length, &query)
                        };
                        assert!(!ptr.is_null());
                        let bytes = unsafe {
                            std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize)
                        };
                        let mut cursor = 0;
                        assert_eq!(
                            crate::ocgcore::utility::read_u16(bytes, &mut cursor, "length")
                                .unwrap(),
                            16
                        );
                        assert_eq!(
                            crate::ocgcore::utility::read_u32(bytes, &mut cursor, "flag").unwrap(),
                            ocgcore_ffi::constants::QUERY_OVERLAY_CARD
                        );
                        let count =
                            crate::ocgcore::utility::read_u32(bytes, &mut cursor, "count").unwrap();
                        assert_eq!(count, 2);
                        for sequence in 0..count {
                            let code =
                                crate::ocgcore::utility::read_u32(bytes, &mut cursor, "code")
                                    .unwrap();
                            assert_eq!(layer.material_at(host, sequence).unwrap().code, code);
                        }
                        host_moved = true;
                    }
                    _ => {}
                }
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert_eq!(attached, 2);
            assert_eq!(detached, 2);
            assert!(host_moved);
            let grave = duel.get_cards(CardLocation::Graveyard);
            for (sequence, core_card) in grave.iter().enumerate() {
                assert_eq!(
                    layer
                        .card_at(CardAddress {
                            controller: Player::Player,
                            location: CardLocation::Graveyard,
                            sequence: sequence as u32,
                        })
                        .unwrap()
                        .code,
                    core_card.unwrap().card_code
                );
            }
            assert_eq!(grave.len(), 3);
            assert!(layer.material_at(host, 0).is_none());
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Overlay test did not reach an input prompt");
}

#[test]
fn tracks_native_material_after_token_host_disappears() {
    fn card_data(code: u32) -> OCG_CardData {
        let mut data = test_card_data(code);
        if code == 100 {
            data.r#type |= ocgcore_ffi::constants::TYPE_TOKEN;
        }
        data
    }

    let core = futures::executor::block_on(OCGCore::load(
        card_data,
        Box::new(|_| None),
        super::super::callbacks::log_core_message,
    ))
    .unwrap();
    let duel = core.create_duel(0).unwrap();
    let mut layer = CardLayer::default();
    let field = |sequence| CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence,
    };
    for (code, sequence) in [(100, 0), (101, 1)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::MonsterZone,
            sequence,
            POS_FACEUP_ATTACK,
        );
        layer.insert(field(sequence), code).unwrap();
    }
    let host = layer.card_at(field(0)).unwrap().id;
    let material = layer.card_at(field(1)).unwrap().id;
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local host = Duel.GetFieldCard(0, 4, 0)
            local material = Duel.GetFieldCard(0, 4, 1)
            Duel.Overlay(host, material)
            Duel.SendtoGrave(host, 0x40)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "overlay-token-test.lua",
    )
    .unwrap();

    duel.start();
    let mut host_disappeared = false;
    let mut material_detached = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if let CoreMessage::Move(data) = &message {
                layer.apply(&message).unwrap();
                if data.card_code == 100 && data.current.location == CardLocationKind::None {
                    host_disappeared = true;
                    assert_eq!(layer.material_at(host, 0).unwrap().id, material);
                }
                if data.card_code == 101
                    && data.previous.location == (CardLocationKind::Overlay { host: None })
                {
                    assert!(data.previous.controller.is_none());
                    assert!(host_disappeared);
                    assert_eq!(
                        layer
                            .card_at(CardAddress {
                                controller: Player::Player,
                                location: CardLocation::Graveyard,
                                sequence: data.current.sequence,
                            })
                            .unwrap()
                            .id,
                        material
                    );
                    material_detached = true;
                }
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert!(material_detached);
            assert_eq!(
                duel.get_cards(CardLocation::Graveyard)[0]
                    .unwrap()
                    .card_code,
                101
            );
            assert!(layer.material_at(host, 0).is_none());
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Token overlay test did not reach an input prompt");
}

#[test]
fn tracks_native_swap_with_overlay_materials() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let field = |sequence| CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence,
    };
    let mut layer = CardLayer::default();
    let mut ids = Vec::new();
    for (code, sequence) in [(100, 0), (101, 1), (102, 2), (103, 3)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::MonsterZone,
            sequence,
            POS_FACEUP_ATTACK,
        );
        ids.push(layer.insert(field(sequence), code).unwrap());
    }
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local first = Duel.GetFieldCard(0, 4, 0)
            local second = Duel.GetFieldCard(0, 4, 1)
            Duel.Overlay(first, Duel.GetFieldCard(0, 4, 2))
            Duel.Overlay(second, Duel.GetFieldCard(0, 4, 3))
            Duel.SwapSequence(first, second)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "swap-test.lua",
    )
    .unwrap();

    duel.start();
    let mut swaps = 0;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if matches!(message, CoreMessage::Move(_) | CoreMessage::Swap(_)) {
                layer.apply(&message).unwrap();
            }
            if matches!(message, CoreMessage::Swap(_)) {
                swaps += 1;
                let queried = duel.get_cards(CardLocation::MonsterZone);
                for (sequence, id, code, material) in
                    [(0, ids[1], 101, ids[3]), (1, ids[0], 100, ids[2])]
                {
                    let card = layer.card_at(field(sequence)).unwrap();
                    assert_eq!((card.id, card.code), (id, code));
                    assert_eq!(queried[sequence as usize].unwrap().card_code, code);
                    assert_eq!(layer.material_at(id, 0).unwrap().id, material);
                }
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert_eq!(swaps, 1);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Swap test did not reach an input prompt");
}

#[test]
fn tracks_native_control_swap() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let field = |controller, sequence| CardAddress {
        controller,
        location: CardLocation::MonsterZone,
        sequence,
    };
    let mut layer = CardLayer::default();
    let mut ids = Vec::new();
    for (team, code, sequence) in [(0, 100, 1), (1, 200, 3)] {
        let player = Player::try_from(team).unwrap();
        let card = OCG_NewCardInfo {
            team,
            duelist: 0,
            code,
            con: team,
            loc: CardLocation::MonsterZone as u32,
            seq: sequence,
            pos: POS_FACEUP_ATTACK,
        };
        unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
        ids.push(layer.insert(field(player, sequence), code).unwrap());
    }
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local first = Duel.GetFieldCard(0, 4, 1)
            local second = Duel.GetFieldCard(1, 4, 3)
            Duel.SwapControl(first, second)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "swap-control-test.lua",
    )
    .unwrap();

    duel.start();
    let mut selections = 0;
    let mut swaps = 0;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::SelectPlace(data) => {
                    let sequence = if data.player == 0 { 1 } else { 3 };
                    assert!(
                        data.zones.iter().any(|zone| {
                            zone.location == CardLocation::MonsterZone && zone.sequence == sequence
                        }),
                        "Original zone is unavailable for control swap"
                    );
                    duel.set_response(Response::Place {
                        controller: data.player,
                        location: CardLocation::MonsterZone as u8,
                        index: sequence,
                    });
                    selections += 1;
                }
                CoreMessage::Swap(_) => {
                    layer.apply(&message).unwrap();
                    swaps += 1;
                    assert_eq!(layer.card_at(field(Player::Player, 1)).unwrap().id, ids[1]);
                    assert_eq!(
                        layer.card_at(field(Player::Opponent, 3)).unwrap().id,
                        ids[0]
                    );
                    assert_eq!(
                        duel.get_cards(CardLocation::MonsterZone)[1]
                            .unwrap()
                            .card_code,
                        200
                    );
                    let query = ocgcore_ffi::types::OCG_QueryInfo {
                        flags: u32::MAX,
                        con: Player::Opponent as u8,
                        loc: CardLocation::MonsterZone as u32,
                        seq: 0,
                        overlay_seq: 0,
                    };
                    let mut length = 0;
                    let ptr = unsafe {
                        ocgcore_ffi::OCG_DuelQueryLocation(duel.handle(), &mut length, &query)
                    };
                    assert!(!ptr.is_null());
                    let bytes =
                        unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize) };
                    let cards = crate::ocgcore::query::decode_location(
                        bytes,
                        Player::Opponent,
                        CardLocation::MonsterZone,
                    )
                    .unwrap();
                    assert_eq!(cards[3].unwrap().card_code, 100);
                }
                _ => {}
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) && swaps > 0 {
            assert_eq!(selections, 2);
            assert_eq!(swaps, 1);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Control swap test did not reach an input prompt");
}

#[test]
fn tracks_native_position_changes_and_reveals() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let field = CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
    };
    duel.add_card(
        Player::Player,
        100,
        Player::Player,
        CardLocation::MonsterZone,
        0,
        POS_FACEUP_ATTACK,
    );
    let mut layer = CardLayer::default();
    let id = layer.insert(field, 100).unwrap();
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local card = Duel.GetFieldCard(0, 4, 0)
            Duel.ChangePosition(card, 4)
            Duel.ChangePosition(card, 8)
            Duel.ChangePosition(card, 1)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "position-test.lua",
    )
    .unwrap();

    duel.start();
    let mut changes = Vec::new();
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if let CoreMessage::PositionChange(data) = &message {
                layer.apply(&message).unwrap();
                let tracked = layer.card_at(field).unwrap();
                assert_eq!(tracked.id, id);
                assert_eq!(tracked.position, Some(data.current_position));
                let queried = duel.get_cards(CardLocation::MonsterZone)[0].unwrap();
                assert_eq!(
                    queried.position.unwrap() as u32,
                    data.current_position.bits()
                );
                changes.push((data.previous_position, data.current_position));
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert_eq!(
                changes,
                [
                    (
                        CardPositionFlags::FACE_UP_ATTACK,
                        CardPositionFlags::FACE_UP_DEFENSE
                    ),
                    (
                        CardPositionFlags::FACE_UP_DEFENSE,
                        CardPositionFlags::FACE_DOWN_DEFENSE
                    ),
                    (
                        CardPositionFlags::FACE_DOWN_DEFENSE,
                        CardPositionFlags::FACE_UP_ATTACK
                    ),
                ]
            );
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Position test did not reach an input prompt");
}

#[test]
fn tracks_native_flip_summon() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let field = CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
    };
    duel.add_card(
        Player::Player,
        100,
        Player::Player,
        CardLocation::MonsterZone,
        0,
        ocgcore_ffi::constants::POS_FACEDOWN_DEFENSE,
    );
    let mut layer = CardLayer::default();
    let id = layer.insert(field, 100).unwrap();
    duel.start();
    let mut requested = false;
    let mut flipping = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::Idle(data) if !requested => {
                    let card = data
                        .battle_positions
                        .iter()
                        .find(|card| card.card_code == 100)
                        .expect("Expected a flip-summon action");
                    let response = [2u8, 0, card.action_index.unwrap(), 0];
                    unsafe {
                        ocgcore_ffi::OCG_DuelSetResponse(
                            duel.handle(),
                            response.as_ptr().cast(),
                            response.len() as u32,
                        )
                    };
                    requested = true;
                }
                CoreMessage::FlipSummoning(_) => {
                    layer.apply(&message).unwrap();
                    let tracked = layer.card_at(field).unwrap();
                    assert_eq!(tracked.id, id);
                    assert_eq!(tracked.position, Some(CardPositionFlags::FACE_UP_ATTACK));
                    flipping = true;
                }
                CoreMessage::FlipSummoned => {
                    assert!(flipping);
                    assert_eq!(
                        duel.get_cards(CardLocation::MonsterZone)[0]
                            .unwrap()
                            .position,
                        Some(BattlePosition::FaceUpAttack)
                    );
                    return;
                }
                CoreMessage::SelectChain(_) => duel.set_response(Response::PassPriority),
                _ => {}
            }
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Flip summon was not completed");
}

#[test]
fn script_loading_reports_lua_errors() {
    let core = test_core();
    let duel = core.create_duel(5).unwrap();
    assert!(
        core.load_script(&duel, b"error('test')", "test.lua")
            .is_err()
    );
}
