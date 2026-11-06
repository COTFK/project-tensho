use std::ffi::CString;
use std::ffi::c_void;
use std::rc::Rc;

use anyhow::{Context, anyhow};
use ocgcore_ffi::constants::DUEL_MODE_MR5;
use ocgcore_ffi::types::OCG_Duel;
use ocgcore_ffi::types::OCG_DuelOptions;
use ocgcore_ffi::types::OCG_Player;

use super::callbacks::CardReaderFn;
use super::callbacks::LogHandlerFn;
use super::callbacks::ScriptReaderFn;
use super::callbacks::card_reader_trampoline;
use super::callbacks::log_handler_trampoline;
use super::callbacks::script_reader_trampoline;
pub use super::duel::Duel;

/// The internal structure that our FFI `payload` will point to.
/// This groups everything the trampolines need to do their job.
pub(super) struct DuelContext {
    pub script_reader: ScriptReaderFn,
    pub card_reader: CardReaderFn,
    pub log_handler: LogHandlerFn,
}

#[derive(Clone)]
pub struct OCGCore {
    _context: Rc<DuelContext>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::ocgcore::CardType;
    use crate::ocgcore::Response;
    use crate::ocgcore::constants::{
        BattlePosition, CardLocation, Phase, Player, WinReason, Winner,
    };
    use crate::ocgcore::location_info::CardLocationKind;
    use crate::ocgcore::messages::{CoreMessage, Hint, HintMessageData, WinMessageData};
    use crate::ui::card_layer::{CardAddress, CardLayer};
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
                            for (sequence, core_card) in duel.get_cards(location).iter().enumerate()
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
                                    .filter(|card| {
                                        card.address.controller == Player::Player
                                            && card.address.location == location
                                    })
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
            if let (Some(moved), Some(summoning), true) =
                (moved_location, summoning_location, summoned)
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
    fn script_loading_reports_lua_errors() {
        let core = test_core();
        let duel = core.create_duel(5).unwrap();
        assert!(
            core.load_script(&duel, b"error('test')", "test.lua")
                .is_err()
        );
    }
}

impl OCGCore {
    pub async fn load(
        card_reader: CardReaderFn,
        script_reader: ScriptReaderFn,
        log_handler: LogHandlerFn,
    ) -> anyhow::Result<Self> {
        ocgcore_ffi::initialize().await;

        let _context = Rc::new(DuelContext {
            card_reader,
            script_reader,
            log_handler,
        });

        Ok(Self { _context })
    }

    pub fn script(&self, name: &str) -> Option<Vec<u8>> {
        (self._context.script_reader)(name)
    }

    pub fn create_duel(&self, starting_draw_count: u32) -> anyhow::Result<Duel> {
        let mut duel: OCG_Duel = std::ptr::null_mut();

        let payload_ptr = &*self._context as *const DuelContext as *mut c_void;
        let options = OCG_DuelOptions {
            seed: [
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
            ],
            flags: DUEL_MODE_MR5.into(),
            team1: OCG_Player {
                starting_lp: 8000,
                starting_draw_count,
                draw_count_per_turn: 1,
            },
            team2: OCG_Player {
                starting_lp: 8000,
                starting_draw_count: 5,
                draw_count_per_turn: 1,
            },
            cardReader: Some(card_reader_trampoline),
            payload1: payload_ptr,
            scriptReader: Some(script_reader_trampoline),
            payload2: payload_ptr,
            logHandler: Some(log_handler_trampoline),
            payload3: payload_ptr,
            cardReaderDone: None,
            payload4: std::ptr::null_mut(),
            enableUnsafeLibraries: 0,
        };

        let status = unsafe { ocgcore_ffi::OCG_CreateDuel(&mut duel, &options) };
        if status != 0 {
            return Err(anyhow!("_OCG_CreateDuel failed with status {status}"));
        }

        Ok(Duel::new(duel, self))
    }

    pub fn load_script(&self, duel: &Duel, script: &[u8], name: &str) -> anyhow::Result<()> {
        let length = u32::try_from(script.len()).context("Script is too large")?;
        let script_name = CString::new(name).context("Script name contains a NUL byte")?;
        let status = unsafe {
            ocgcore_ffi::OCG_LoadScript(
                duel.handle(),
                script.as_ptr().cast(),
                length,
                script_name.as_ptr(),
            )
        };
        if status == 0 {
            return Err(anyhow!("Failed to load script {name}"));
        }
        Ok(())
    }
}
