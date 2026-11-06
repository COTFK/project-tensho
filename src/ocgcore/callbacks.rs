use std::ffi::CStr;
use std::ffi::c_char;
use std::ffi::c_void;

use ocgcore_ffi::enums::OCG_LogTypes;
use ocgcore_ffi::types::OCG_CardData;
use ocgcore_ffi::types::OCG_Duel;

use super::core::DuelContext;

pub type CardReaderFn = fn(code: u32) -> OCG_CardData;
pub type ScriptReaderFn = Box<dyn Fn(&str) -> Option<Vec<u8>>>;
pub type LogHandlerFn = fn(message: String, log_type: i32);

pub fn log_core_message(message: String, log_type: i32) {
    match OCG_LogTypes::try_from(log_type) {
        Ok(OCG_LogTypes::OCG_LOG_TYPE_ERROR) => tracing::error!(target: "ocgcore", "{message}"),
        Ok(OCG_LogTypes::OCG_LOG_TYPE_FROM_SCRIPT | OCG_LogTypes::OCG_LOG_TYPE_FOR_DEBUG) => {
            tracing::debug!(target: "ocgcore", "{message}")
        }
        Ok(OCG_LogTypes::OCG_LOG_TYPE_UNDEFINED) | Err(_) => {
            tracing::warn!(target: "ocgcore", log_type, "{message}")
        }
    }
}

pub unsafe extern "C" fn log_handler_trampoline(
    payload: *mut c_void,
    string: *const c_char,
    log_type: i32,
) {
    if payload.is_null() || string.is_null() {
        return;
    }
    let context = unsafe { &*(payload as *const DuelContext) };
    let string = unsafe { CStr::from_ptr(string).to_str().unwrap() };

    (context.log_handler)(string.to_string(), log_type);
}

pub unsafe extern "C" fn card_reader_trampoline(
    payload: *mut c_void,
    code: u32,
    data: *mut OCG_CardData,
) {
    if payload.is_null() || data.is_null() {
        return;
    }
    let context = unsafe { &*(payload as *const DuelContext) };

    let card_info = (context.card_reader)(code);
    unsafe { std::ptr::write(data, card_info) };
}

pub unsafe extern "C" fn script_reader_trampoline(
    payload: *mut c_void,
    duel: OCG_Duel,
    name: *const c_char,
) -> i32 {
    if payload.is_null() || name.is_null() || duel.is_null() {
        return 0;
    }

    let context = unsafe { &*(payload as *const DuelContext) };
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if let Some(script_bytes) = (context.script_reader)(name_str) {
        return unsafe {
            ocgcore_ffi::OCG_LoadScript(
                duel,
                script_bytes.as_ptr() as *const c_char,
                script_bytes.len() as u32,
                name,
            )
        };
    }

    0 // Script not found
}

#[cfg(test)]
#[path = "../../tests/unit/ocgcore/callbacks.rs"]
mod tests;
