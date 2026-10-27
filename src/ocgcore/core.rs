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
    use crate::ocgcore::constants::{CardLocation, Phase, WinReason, Winner};
    use crate::ocgcore::messages::{CoreMessage, WinMessageData};
    use ocgcore_ffi::constants::{TYPE_MONSTER, TYPE_NORMAL};
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
            |message| tracing::error!("{message}"),
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
    fn sandbox_continues_after_either_player_decks_out() {
        for (player_cards, opponent_cards, winner) in
            [(5, 0, Winner::Player), (0, 5, Winner::Opponent)]
        {
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
        assert!(matches!(messages.last(), Some(CoreMessage::Idle(_))));
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
