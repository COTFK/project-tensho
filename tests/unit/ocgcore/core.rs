use super::*;
use crate::duel::cards::{Card, CardAddress, CardId, CardPlacement, CardTracker};
use crate::ocgcore::CardType;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Phase, Player, WinReason, Winner};
use crate::ocgcore::location_info::{CardLocationKind, CardPositionFlags};
use crate::ocgcore::messages::{CoreMessage, Hint, HintMessageData, WinMessageData};
use ocgcore_ffi::constants::{POS_FACEUP_ATTACK, TYPE_MONSTER, TYPE_NORMAL};
use ocgcore_ffi::enums::OCG_DuelStatus;
use ocgcore_ffi::types::{OCG_CardData, OCG_NewCardInfo};

trait Materials {
    fn material_at(&self, host: CardId, sequence: u32) -> Option<&Card>;
}

impl Materials for CardTracker {
    fn material_at(&self, host: CardId, sequence: u32) -> Option<&Card> {
        self.cards()
            .iter()
            .find(|card| card.placement == CardPlacement::Overlay { host, sequence })
    }
}

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
fn tracks_native_deck_reversal_top_confirmations_and_draw() {
    fn card_data(code: u32) -> OCG_CardData {
        let mut data = test_card_data(code);
        if (300..=302).contains(&code) {
            data.r#type = TYPE_MONSTER | ocgcore_ffi::constants::TYPE_FUSION;
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
    let mut layer = CardTracker::default();
    for (player, location, count, base) in [
        (Player::Player, CardLocation::Deck, 12, 100),
        (Player::Opponent, CardLocation::Deck, 12, 200),
        (Player::Player, CardLocation::ExtraDeck, 3, 300),
    ] {
        for sequence in 0..count {
            let code = base + sequence;
            let card = OCG_NewCardInfo {
                team: player as u8,
                duelist: 0,
                code,
                con: player as u8,
                loc: location as u32,
                seq: 0,
                pos: ocgcore_ffi::constants::POS_FACEDOWN,
            };
            unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
            layer
                .insert(
                    CardAddress {
                        controller: player,
                        location,
                        sequence,
                    },
                    code,
                )
                .unwrap();
        }
    }
    core.load_script(
        &duel,
        br#"
            local reverse = Effect.GlobalEffect()
            reverse:SetType(0x0002)
            reverse:SetCode(294)
            Duel.RegisterEffect(reverse, 0)
            local effect = Effect.GlobalEffect()
            effect:SetType(0x0802)
            effect:SetCode(1113)
            effect:SetOperation(function()
                Duel.ConfirmDecktop(0, 2)
                Duel.ConfirmExtratop(0, 2)
                Duel.Draw(0, 1, 0x40)
                reverse:Reset()
            end)
            Duel.RegisterEffect(effect, 0)
            "#,
        "reverse-deck-test.lua",
    )
    .unwrap();
    duel.start();
    let mut reversals = 0;
    let mut top_offset = false;
    let mut confirmations = [false; 2];
    let mut draw_after_reverse = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::ReverseDeck => reversals += 1,
                CoreMessage::DeckTop(data) if data.offset > 0 => top_offset = true,
                CoreMessage::ConfirmDeckTop(_) => confirmations[0] = true,
                CoreMessage::ConfirmExtraTop(_) => confirmations[1] = true,
                CoreMessage::Draw(_) if reversals > 0 => draw_after_reverse = true,
                _ => {}
            }
            if matches!(
                message,
                CoreMessage::ReverseDeck
                    | CoreMessage::DeckTop(_)
                    | CoreMessage::ConfirmDeckTop(_)
                    | CoreMessage::ConfirmExtraTop(_)
                    | CoreMessage::Draw(_)
                    | CoreMessage::Move(_)
            ) {
                layer
                    .apply(&message)
                    .unwrap_or_else(|error| panic!("{message:?}: {error:#}"));
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) && confirmations[1] {
            assert_eq!(reversals, 2);
            assert!(top_offset);
            assert_eq!(confirmations, [true, true]);
            assert!(draw_after_reverse);
            for player in [Player::Player, Player::Opponent] {
                let query = ocgcore_ffi::types::OCG_QueryInfo {
                    flags: u32::MAX,
                    con: player as u8,
                    loc: CardLocation::Deck as u32,
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
                let cards =
                    crate::ocgcore::query::decode_location(bytes, player, CardLocation::Deck)
                        .unwrap();
                assert_eq!(
                        layer.cards().iter().filter(|card| matches!(
                            card.placement,
                            CardPlacement::Zone(address)
                                if address.controller == player && address.location == CardLocation::Deck
                        )).count(),
                        cards.len()
                    );
                for (sequence, card) in cards.iter().enumerate() {
                    assert_eq!(
                        layer
                            .card_at(CardAddress {
                                controller: player,
                                location: CardLocation::Deck,
                                sequence: sequence as u32,
                            })
                            .unwrap()
                            .code,
                        card.unwrap().card_code
                    );
                }
            }
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Deck reversal test did not reach an input prompt");
}

#[test]
fn tracks_native_graveyard_deck_exchange_and_shuffle() {
    fn card_data(code: u32) -> OCG_CardData {
        let mut data = test_card_data(code);
        if (300..=302).contains(&code) {
            data.r#type = TYPE_MONSTER | ocgcore_ffi::constants::TYPE_FUSION;
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
    let mut layer = CardTracker::default();
    for (player, location, codes) in [
        (Player::Player, CardLocation::Deck, vec![1; 12]),
        (Player::Opponent, CardLocation::Deck, vec![2; 12]),
        (
            Player::Player,
            CardLocation::Graveyard,
            vec![10, 300, 10, 301, 11],
        ),
        (Player::Player, CardLocation::ExtraDeck, vec![302]),
    ] {
        for (sequence, code) in codes.into_iter().enumerate() {
            let card = OCG_NewCardInfo {
                team: player as u8,
                duelist: 0,
                code,
                con: player as u8,
                loc: location as u32,
                seq: 0,
                pos: ocgcore_ffi::constants::POS_FACEDOWN,
            };
            unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
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
            Duel.SwapDeckAndGrave(0)
        end)
        Duel.RegisterEffect(effect, 0)
        "#,
        "swap-grave-deck-test.lua",
    )
    .unwrap();
    duel.start();
    let mut exchanged = false;
    let mut shuffled = false;
    let mut moved_ids = Vec::new();
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::SwapGraveDeck(data) => {
                    assert_eq!(data.extra_insert_index, 1);
                    assert_eq!(data.extra_cards, [0b0000_1010]);
                    layer.apply(&message).unwrap();
                    let at = |location, sequence| CardAddress {
                        controller: Player::Player,
                        location,
                        sequence,
                    };
                    assert_eq!(
                        (0..3)
                            .map(|i| layer.card_at(at(CardLocation::Deck, i)).unwrap().code)
                            .collect::<Vec<_>>(),
                        [10, 10, 11]
                    );
                    assert_eq!(
                        (0..3)
                            .map(|i| layer.card_at(at(CardLocation::ExtraDeck, i)).unwrap().code)
                            .collect::<Vec<_>>(),
                        [302, 300, 301]
                    );
                    moved_ids = (0..3)
                        .map(|i| layer.card_at(at(CardLocation::Deck, i)).unwrap().id)
                        .collect();
                    exchanged = true;
                }
                CoreMessage::ShuffleDeck(data) if exchanged && data.player == Player::Player => {
                    layer.apply(&message).unwrap();
                    for sequence in 0..3 {
                        let card = layer
                            .card_at(CardAddress {
                                controller: Player::Player,
                                location: CardLocation::Deck,
                                sequence,
                            })
                            .unwrap();
                        assert!(!moved_ids.contains(&card.id));
                    }
                    shuffled = true;
                }
                CoreMessage::Draw(_) | CoreMessage::Move(_) | CoreMessage::ShuffleDeck(_) => {
                    layer.apply(&message).unwrap();
                }
                _ => {}
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) && exchanged {
            assert!(shuffled);
            for location in [
                CardLocation::Deck,
                CardLocation::Graveyard,
                CardLocation::ExtraDeck,
            ] {
                let core_cards = duel.query_cards(Player::Player, location).unwrap();
                let tracked: Vec<_> = layer.cards().iter().filter(|card| matches!(
                    card.placement,
                    CardPlacement::Zone(address) if address.controller == Player::Player && address.location == location
                )).collect();
                assert_eq!(tracked.len(), core_cards.len());
                if location != CardLocation::Deck {
                    for (sequence, card) in core_cards.iter().enumerate() {
                        assert_eq!(
                            layer
                                .card_at(CardAddress {
                                    controller: Player::Player,
                                    location,
                                    sequence: sequence as u32
                                })
                                .unwrap()
                                .code,
                            card.unwrap().card_code
                        );
                    }
                }
            }
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Exchange test did not reach an input prompt");
}

#[test]
fn native_exchange_single_survivor_retains_its_old_sequence() {
    fn card_data(code: u32) -> OCG_CardData {
        let mut data = test_card_data(code);
        if code == 300 {
            data.r#type = TYPE_MONSTER | ocgcore_ffi::constants::TYPE_FUSION;
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
    for (team, location, codes) in [
        (0, CardLocation::Deck, vec![1; 12]),
        (1, CardLocation::Deck, vec![2; 12]),
        (0, CardLocation::Graveyard, vec![300, 10]),
    ] {
        for code in codes {
            let card = OCG_NewCardInfo {
                team,
                duelist: 0,
                code,
                con: team,
                loc: location as u32,
                seq: 0,
                pos: ocgcore_ffi::constants::POS_FACEDOWN,
            };
            unsafe { ocgcore_ffi::OCG_DuelNewCard(duel.handle(), &card) };
        }
    }
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            Duel.SwapDeckAndGrave(0)
            Duel.SendtoGrave(Duel.GetFieldCard(0, 1, 0), 0x40)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "exchange-single.lua",
    )
    .unwrap();
    duel.start();
    let mut found = false;
    let mut rejected = false;
    let mut layer = CardTracker::default();
    for (sequence, code) in [300, 10].into_iter().enumerate() {
        layer
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Graveyard,
                    sequence: sequence as u32,
                },
                code,
            )
            .unwrap();
    }
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if matches!(message, CoreMessage::SwapGraveDeck(_)) {
                let before = layer.clone();
                assert!(
                    layer
                        .apply(&message)
                        .unwrap_err()
                        .to_string()
                        .contains("stale sequence")
                );
                assert_eq!(layer, before);
                rejected = true;
            }
            if let CoreMessage::Move(data) = message {
                if data.card_code == 10
                    && data.previous.location == CardLocationKind::Known(CardLocation::Deck)
                {
                    assert_eq!(data.previous.sequence, 1);
                    found = true;
                }
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) && found {
            assert!(rejected);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Single survivor movement was not reported");
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

    let monsters = duel
        .query_cards(Player::Player, CardLocation::MonsterZone)
        .unwrap();
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
    assert_eq!(
        duel.query_cards(Player::Player, CardLocation::Graveyard)
            .unwrap(),
        vec![]
    );
    let hand = duel
        .query_cards(Player::Player, CardLocation::Hand)
        .unwrap();
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].unwrap().sequence, 0);
    assert_eq!(hand[0].unwrap().card_code, 103);
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
    let mut layer = CardTracker::default();
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
                        for (sequence, core_card) in duel
                            .query_cards(Player::Player, location)
                            .unwrap()
                            .iter()
                            .enumerate()
                        {
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
                            unsafe {
                                ocgcore_ffi::OCG_DuelQueryCount(
                                    duel.handle(),
                                    Player::Player as u8,
                                    location as u32,
                                )
                            } as usize
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
            let queried = duel
                .query_cards(Player::Player, CardLocation::MonsterZone)
                .unwrap();
            assert_eq!(
                tracked.code,
                queried[moved.sequence as usize].unwrap().card_code
            );
            for (sequence, core_card) in duel
                .query_cards(Player::Player, CardLocation::Hand)
                .unwrap()
                .iter()
                .enumerate()
            {
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
    let mut layer = CardTracker::default();
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
            let grave = duel
                .query_cards(Player::Player, CardLocation::Graveyard)
                .unwrap();
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
fn tracks_native_set_card_shuffle_with_materials() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let field = |sequence| CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence,
    };
    let unaffected = CardAddress {
        controller: Player::Player,
        location: CardLocation::SpellTrapZone,
        sequence: 0,
    };
    let mut layer = CardTracker::default();
    let mut ids = Vec::new();
    for (code, sequence) in [(100, 0), (101, 1), (102, 2), (103, 3), (104, 4), (105, 5)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::MonsterZone,
            sequence,
            if sequence < 4 {
                ocgcore_ffi::constants::POS_FACEDOWN_DEFENSE
            } else {
                POS_FACEUP_ATTACK
            },
        );
        ids.push(layer.insert(field(sequence), code).unwrap());
    }
    duel.add_card(
        Player::Player,
        106,
        Player::Player,
        CardLocation::SpellTrapZone,
        0,
        POS_FACEUP_ATTACK,
    );
    let unaffected_id = layer.insert(unaffected, 106).unwrap();
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local first = Duel.GetFieldCard(0, 4, 0)
            local second = Duel.GetFieldCard(0, 4, 2)
            local material1 = Duel.GetFieldCard(0, 4, 4)
            local material2 = Duel.GetFieldCard(0, 4, 5)
            Duel.Overlay(first, material1)
            Duel.Overlay(second, material2)
            local group = Group.CreateGroup()
            for i=0,3 do group:AddCard(Duel.GetFieldCard(0, 4, i)) end
            Duel.ShuffleSetCard(group)
            Duel.SendtoGrave(material1, 0x40)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "set-shuffle-test.lua",
    )
    .unwrap();

    duel.start();
    let mut shuffled = false;
    let mut detached = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if matches!(
                message,
                CoreMessage::Move(_) | CoreMessage::ShuffleSetCard(_)
            ) {
                layer.apply(&message).unwrap();
            }
            match &message {
                CoreMessage::ShuffleSetCard(data) => {
                    assert_eq!(data.previous.len(), 4);
                    assert_eq!(data.current.iter().flatten().count(), 2);
                    let queried = duel
                        .query_cards(Player::Player, CardLocation::MonsterZone)
                        .unwrap();
                    for (previous, current) in data.previous.iter().zip(&data.current) {
                        let source_id = ids[previous.sequence as usize];
                        if let Some(current) = current {
                            let tracked = layer.card_at(field(current.sequence)).unwrap();
                            assert_eq!(tracked.id, source_id);
                            assert_eq!(
                                tracked.code,
                                queried[current.sequence as usize].unwrap().card_code
                            );
                        }
                    }
                    for sequence in 0..4 {
                        let card = layer.card_at(field(sequence)).unwrap();
                        if card.id != ids[0] && card.id != ids[2] {
                            assert!(!ids[..4].contains(&card.id));
                            assert_eq!(card.code, 0);
                        }
                    }
                    assert_eq!(layer.material_at(ids[0], 0).unwrap().id, ids[4]);
                    assert_eq!(layer.material_at(ids[2], 0).unwrap().id, ids[5]);
                    assert_eq!(layer.card_at(unaffected).unwrap().id, unaffected_id);
                    shuffled = true;
                }
                CoreMessage::Move(data) if data.card_code == 104 && shuffled => {
                    let grave = CardAddress {
                        controller: Player::Player,
                        location: CardLocation::Graveyard,
                        sequence: data.current.sequence,
                    };
                    assert_eq!(layer.card_at(grave).unwrap().id, ids[4]);
                    assert!(layer.material_at(ids[0], 0).is_none());
                    assert_eq!(layer.material_at(ids[2], 0).unwrap().id, ids[5]);
                    detached = true;
                }
                _ => {}
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert!(shuffled);
            assert!(detached);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Set-card shuffle test did not reach an input prompt");
}

#[test]
fn tracks_native_group_spell_sets_with_hidden_destinations() {
    fn card_data(code: u32) -> OCG_CardData {
        let mut data = test_card_data(code);
        if (201..=203).contains(&code) {
            data.r#type = ocgcore_ffi::constants::TYPE_SPELL;
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
    let address = |location, sequence| CardAddress {
        controller: Player::Player,
        location,
        sequence,
    };
    let mut layer = CardTracker::default();
    for (sequence, code) in [201, 202, 203].into_iter().enumerate() {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::Hand,
            sequence as u32,
            ocgcore_ffi::constants::POS_FACEDOWN,
        );
        layer
            .insert(address(CardLocation::Hand, sequence as u32), code)
            .unwrap();
    }
    duel.add_card(
        Player::Player,
        100,
        Player::Player,
        CardLocation::MonsterZone,
        0,
        POS_FACEUP_ATTACK,
    );
    let unaffected = layer
        .insert(address(CardLocation::MonsterZone, 0), 100)
        .unwrap();
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            local group = Duel.GetFieldGroup(0, 2, 0)
            Duel.SSet(0, group, 0, false)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "group-set-shuffle-test.lua",
    )
    .unwrap();

    duel.start();
    let mut selections = 0;
    let mut shuffled = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::SelectPlace(data) => {
                    let zone = data
                        .zones
                        .iter()
                        .rev()
                        .find(|zone| {
                            zone.location == CardLocation::SpellTrapZone && zone.sequence < 5
                        })
                        .expect("No available spell/trap zone");
                    duel.set_response(Response::Place {
                        controller: data.player,
                        location: zone.location as u8,
                        index: zone.sequence,
                    });
                    selections += 1;
                }
                CoreMessage::Move(_) | CoreMessage::ShuffleHand(_) | CoreMessage::SetCard(_) => {
                    layer.apply(&message).unwrap();
                }
                CoreMessage::ShuffleSetCard(data) => {
                    assert_eq!(data.location, CardLocation::SpellTrapZone);
                    assert_eq!(data.previous.len(), 3);
                    assert!(data.current.iter().all(Option::is_none));
                    let old_ids: Vec<_> = data
                        .previous
                        .iter()
                        .map(|info| {
                            layer
                                .card_at(address(CardLocation::SpellTrapZone, info.sequence))
                                .unwrap()
                                .id
                        })
                        .collect();
                    layer.apply(&message).unwrap();
                    let queried = duel
                        .query_cards(Player::Player, CardLocation::SpellTrapZone)
                        .unwrap();
                    for info in &data.previous {
                        let card = layer
                            .card_at(address(CardLocation::SpellTrapZone, info.sequence))
                            .unwrap();
                        assert!(!old_ids.contains(&card.id));
                        assert_eq!(card.code, 0);
                        assert!(queried[info.sequence as usize].is_some());
                    }
                    assert_eq!(
                        layer
                            .card_at(address(CardLocation::MonsterZone, 0))
                            .unwrap()
                            .id,
                        unaffected
                    );
                    shuffled = true;
                }
                _ => {}
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) && shuffled {
            assert_eq!(selections, 3);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Group spell set test did not reach an input prompt");
}

#[test]
fn tracks_native_hidden_shuffles_and_subsequent_draw() {
    fn card_data(code: u32) -> OCG_CardData {
        let mut data = test_card_data(code);
        if (201..=203).contains(&code) {
            data.r#type = TYPE_MONSTER | ocgcore_ffi::constants::TYPE_FUSION;
        } else if code == 301 {
            data.r#type |= ocgcore_ffi::constants::TYPE_PENDULUM;
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
    let address = |location, sequence| CardAddress {
        controller: Player::Player,
        location,
        sequence,
    };
    let mut layer = CardTracker::default();
    for (location, codes) in [
        (CardLocation::Deck, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        (CardLocation::Hand, vec![101, 101, 102]),
        (CardLocation::ExtraDeck, vec![201, 202, 203]),
        (CardLocation::MonsterZone, vec![301]),
    ] {
        for (sequence, code) in codes.into_iter().enumerate() {
            let position = if location == CardLocation::MonsterZone {
                POS_FACEUP_ATTACK
            } else {
                ocgcore_ffi::constants::POS_FACEDOWN
            };
            duel.add_card(
                Player::Player,
                code,
                Player::Player,
                location,
                sequence as u32,
                position,
            );
            layer
                .insert(address(location, sequence as u32), code)
                .unwrap();
        }
    }
    let pendulum = layer
        .card_at(address(CardLocation::MonsterZone, 0))
        .unwrap()
        .id;
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            Duel.ShuffleHand(0)
            Duel.ShuffleDeck(0)
            Duel.SendtoGrave(Duel.GetFieldCard(0, 4, 0), 0x40)
            Duel.ShuffleExtra(0)
            Duel.Draw(0, 1, 0x40)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "shuffle-test.lua",
    )
    .unwrap();

    duel.start();
    let mut hand_shuffled = false;
    let mut deck_shuffled = false;
    let mut extra_shuffled = false;
    let mut drew_after_shuffle = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            let shuffled_location = match &message {
                CoreMessage::ShuffleHand(data) if data.player == Player::Player => {
                    Some(CardLocation::Hand)
                }
                CoreMessage::ShuffleDeck(data) if data.player == Player::Player => {
                    Some(CardLocation::Deck)
                }
                CoreMessage::ShuffleExtra(data) if data.player == Player::Player => {
                    Some(CardLocation::ExtraDeck)
                }
                _ => None,
            };
            if let Some(location) = shuffled_location {
                let old_ids: Vec<_> = layer
                    .cards()
                    .iter()
                    .filter(|card| {
                        matches!(card.placement, CardPlacement::Zone(a)
                        if a.controller == Player::Player && a.location == location)
                    })
                    .map(|card| card.id)
                    .collect();
                layer.apply(&message).unwrap();
                let core_cards = duel.query_cards(Player::Player, location).unwrap();
                assert_eq!(core_cards.len(), old_ids.len());
                for (sequence, core_card) in core_cards.iter().enumerate() {
                    let tracked = layer.card_at(address(location, sequence as u32)).unwrap();
                    if location == CardLocation::ExtraDeck && tracked.id == pendulum {
                        assert_eq!(tracked.code, 301);
                        assert_eq!(tracked.position, Some(CardPositionFlags::FACE_UP_DEFENSE));
                    } else {
                        assert!(!old_ids.contains(&tracked.id));
                        if location == CardLocation::Deck {
                            assert_eq!(tracked.code, 0);
                        } else {
                            assert_eq!(tracked.code, core_card.unwrap().card_code);
                        }
                    }
                }
                match location {
                    CardLocation::Hand => hand_shuffled = true,
                    CardLocation::Deck => deck_shuffled = true,
                    CardLocation::ExtraDeck => {
                        assert_eq!(layer.card_at(address(location, 3)).unwrap().id, pendulum);
                        extra_shuffled = true;
                    }
                    _ => unreachable!(),
                }
            } else if matches!(message, CoreMessage::Draw(_) | CoreMessage::Move(_)) {
                if matches!(message, CoreMessage::Draw(_)) && deck_shuffled {
                    drew_after_shuffle = true;
                }
                layer.apply(&message).unwrap();
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert!(hand_shuffled);
            assert!(deck_shuffled);
            assert!(extra_shuffled);
            assert!(drew_after_shuffle);
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Shuffle test did not reach an input prompt");
}

#[test]
fn tracks_native_bulk_removal_and_later_material_move() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let address = |location, sequence| CardAddress {
        controller: Player::Player,
        location,
        sequence,
    };
    let mut layer = CardTracker::default();
    let mut grave_ids = Vec::new();
    for (code, sequence) in [(1, 0), (2, 1), (3, 2)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::Graveyard,
            sequence,
            POS_FACEUP_ATTACK,
        );
        grave_ids.push(
            layer
                .insert(address(CardLocation::Graveyard, sequence), code)
                .unwrap(),
        );
    }
    let mut field_ids = Vec::new();
    for (code, sequence) in [(100, 0), (101, 1)] {
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::MonsterZone,
            sequence,
            POS_FACEUP_ATTACK,
        );
        field_ids.push(
            layer
                .insert(address(CardLocation::MonsterZone, sequence), code)
                .unwrap(),
        );
    }
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
            local group = Group.CreateGroup()
            group:AddCard(Duel.GetFieldCard(0, 16, 0))
            group:AddCard(Duel.GetFieldCard(0, 16, 2))
            group:AddCard(host)
            Duel.RemoveCards(group)
            Duel.SendtoGrave(material, 0x40)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "bulk-remove-test.lua",
    )
    .unwrap();

    duel.start();
    let mut removed = false;
    let mut material_moved = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if matches!(message, CoreMessage::Move(_) | CoreMessage::RemoveCards(_)) {
                layer.apply(&message).unwrap();
            }
            match message {
                CoreMessage::RemoveCards(data) => {
                    assert_eq!(data.cards.len(), 3);
                    assert_eq!(
                        layer
                            .card_at(address(CardLocation::Graveyard, 0))
                            .unwrap()
                            .id,
                        grave_ids[1]
                    );
                    assert!(
                        layer
                            .card_at(address(CardLocation::MonsterZone, 0))
                            .is_none()
                    );
                    assert_eq!(layer.material_at(field_ids[0], 0).unwrap().id, field_ids[1]);
                    removed = true;
                }
                CoreMessage::Move(data)
                    if data.card_code == 101
                        && data.previous.location == (CardLocationKind::Overlay { host: None }) =>
                {
                    assert!(removed);
                    assert_eq!(
                        layer
                            .card_at(address(CardLocation::Graveyard, data.current.sequence))
                            .unwrap()
                            .id,
                        field_ids[1]
                    );
                    material_moved = true;
                }
                _ => {}
            }
        }
        if matches!(status, OCG_DuelStatus::OCG_DUEL_STATUS_AWAITING) {
            assert!(removed);
            assert!(material_moved);
            assert!(layer.material_at(field_ids[0], 0).is_none());
            let core_cards = duel
                .query_cards(Player::Player, CardLocation::Graveyard)
                .unwrap();
            assert_eq!(core_cards.len(), 2);
            for (sequence, core_card) in core_cards.iter().enumerate() {
                assert_eq!(
                    layer
                        .card_at(address(CardLocation::Graveyard, sequence as u32))
                        .unwrap()
                        .code,
                    core_card.unwrap().card_code
                );
            }
            return;
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Bulk removal test did not reach an input prompt");
}

#[test]
fn rejects_native_full_removal_chunk() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let mut layer = CardTracker::default();
    for sequence in 0..255 {
        let code = sequence + 1;
        duel.add_card(
            Player::Player,
            code,
            Player::Player,
            CardLocation::Graveyard,
            sequence,
            POS_FACEUP_ATTACK,
        );
        layer
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Graveyard,
                    sequence,
                },
                code,
            )
            .unwrap();
    }
    core.load_script(
        &duel,
        br#"
        local effect = Effect.GlobalEffect()
        effect:SetType(0x0802)
        effect:SetCode(1113)
        effect:SetOperation(function()
            Duel.RemoveCards(Duel.GetFieldGroup(0, 16, 0))
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "remove-full-chunk-test.lua",
    )
    .unwrap();

    duel.start();
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            if let CoreMessage::RemoveCards(data) = &message {
                assert_eq!(data.cards.len(), 255);
                let before = layer.clone();
                let error = layer.apply(&message).unwrap_err();
                assert!(error.to_string().contains("group boundaries"));
                assert_eq!(layer, before);
                return;
            }
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Core did not emit a full removal chunk");
}

#[test]
fn rejects_native_direct_overlay_removal() {
    let core = test_core();
    let duel = core.create_duel(0).unwrap();
    let field = |sequence| CardAddress {
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence,
    };
    let mut layer = CardTracker::default();
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
            Duel.RemoveCards(material)
        end)
        Duel.RegisterEffect(effect, 0)
    "#,
        "remove-material-test.lua",
    )
    .unwrap();

    duel.start();
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::Move(_) => layer.apply(&message).unwrap(),
                CoreMessage::RemoveCards(_) => {
                    let before = layer.clone();
                    let error = layer.apply(&message).unwrap_err();
                    assert!(
                        error
                            .to_string()
                            .contains("does not remove the material from the core")
                    );
                    assert_eq!(layer, before);
                    assert_eq!(layer.material_at(host, 0).unwrap().id, material);
                    let query = ocgcore_ffi::types::OCG_QueryInfo {
                        flags: ocgcore_ffi::constants::QUERY_OVERLAY_CARD,
                        con: Player::Player as u8,
                        loc: CardLocation::MonsterZone as u32,
                        seq: 0,
                        overlay_seq: 0,
                    };
                    let mut length = 0;
                    let ptr =
                        unsafe { ocgcore_ffi::OCG_DuelQuery(duel.handle(), &mut length, &query) };
                    assert!(!ptr.is_null());
                    let bytes =
                        unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), length as usize) };
                    let mut cursor = 0;
                    crate::ocgcore::utility::read_u16(bytes, &mut cursor, "length").unwrap();
                    crate::ocgcore::utility::read_u32(bytes, &mut cursor, "flag").unwrap();
                    assert_eq!(
                        crate::ocgcore::utility::read_u32(bytes, &mut cursor, "count").unwrap(),
                        1
                    );
                    assert_eq!(
                        crate::ocgcore::utility::read_u32(bytes, &mut cursor, "code").unwrap(),
                        101
                    );
                    return;
                }
                _ => {}
            }
        }
        assert_ne!(status, OCG_DuelStatus::OCG_DUEL_STATUS_END);
    }
    panic!("Core did not emit a direct overlay removal");
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
    let mut layer = CardTracker::default();
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
                duel.query_cards(Player::Player, CardLocation::Graveyard)
                    .unwrap()[0]
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
    let mut layer = CardTracker::default();
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
                let queried = duel
                    .query_cards(Player::Player, CardLocation::MonsterZone)
                    .unwrap();
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
    let mut layer = CardTracker::default();
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
                        duel.query_cards(Player::Player, CardLocation::MonsterZone)
                            .unwrap()[1]
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
    let mut layer = CardTracker::default();
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
                let queried = duel
                    .query_cards(Player::Player, CardLocation::MonsterZone)
                    .unwrap()[0]
                    .unwrap();
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
    let mut layer = CardTracker::default();
    let id = layer.insert(field, 100).unwrap();
    duel.start();
    let mut requested = false;
    let mut flipping = false;
    for _ in 0..1000 {
        let status = duel.process();
        for message in duel.parse_messages() {
            match &message {
                CoreMessage::Idle(_) if !requested => {
                    let response = [2u8, 0, 0, 0];
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
                        duel.query_cards(Player::Player, CardLocation::MonsterZone)
                            .unwrap()[0]
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
