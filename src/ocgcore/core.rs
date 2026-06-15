use std::ffi::CString;
use std::ffi::c_void;
use std::rc::Rc;

use anyhow::anyhow;
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
#[derive(Hash, Debug, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)]
pub(super) struct DuelContext {
    pub script_reader: ScriptReaderFn,
    pub card_reader: CardReaderFn,
    pub log_handler: LogHandlerFn,
}

#[derive(Hash, Debug, Clone, PartialEq)]
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

    pub fn create_duel(&self, starting_draw_count: u32) -> anyhow::Result<Duel> {
        const DUEL_MODE_MR5: u64 = 0x2E800;
        let mut duel: OCG_Duel = std::ptr::null_mut();

        let payload_ptr = &*self._context as *const DuelContext as *mut c_void;
        let options = OCG_DuelOptions {
            seed: [
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
            ],
            flags: DUEL_MODE_MR5,
            team1: OCG_Player {
                starting_lp: 8000,
                starting_draw_count: starting_draw_count,
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

    pub fn load_script(&self, duel: &Duel, script: Vec<u8>, name: &str) -> i32 {
        let length = script.len() as u32;
        let script = CString::new(script).unwrap();
        let script_ptr = script.as_ptr();

        let name = CString::new(name).unwrap();
        let name_ptr = name.as_ptr();

        unsafe { ocgcore_ffi::OCG_LoadScript(duel.handle, script_ptr, length, name_ptr) }
    }
}
