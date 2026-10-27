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
