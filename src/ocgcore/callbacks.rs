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
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fmt;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Level, Metadata, Subscriber};

    #[derive(Debug, Default)]
    struct RecordedFields {
        message: String,
        log_type: Option<i64>,
        bytes: Option<String>,
    }

    impl Visit for RecordedFields {
        fn record_i64(&mut self, field: &Field, value: i64) {
            if field.name() == "log_type" {
                self.log_type = Some(value);
            }
        }

        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            match field.name() {
                "message" => self.message = format!("{value:?}"),
                "bytes" => self.bytes = Some(format!("{value:?}")),
                _ => {}
            }
        }
    }

    #[derive(Clone, Default)]
    struct RecordingSubscriber {
        events: Arc<Mutex<Vec<(Level, RecordedFields)>>>,
    }

    impl Subscriber for RecordingSubscriber {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }

        fn record(&self, _: &Id, _: &Record<'_>) {}

        fn record_follows_from(&self, _: &Id, _: &Id) {}

        fn event(&self, event: &Event<'_>) {
            let mut fields = RecordedFields::default();
            event.record(&mut fields);
            self.events
                .lock()
                .unwrap()
                .push((*event.metadata().level(), fields));
        }

        fn enter(&self, _: &Id) {}

        fn exit(&self, _: &Id) {}
    }

    #[test]
    fn callback_preserves_core_log_severity_and_message() {
        let subscriber = RecordingSubscriber::default();
        let events = subscriber.events.clone();
        let mut context = DuelContext {
            card_reader: |_| panic!("Unexpected card lookup"),
            script_reader: Box::new(|_| None),
            log_handler: log_core_message,
        };
        let message = CString::new("core message").unwrap();
        tracing::subscriber::with_default(subscriber, || {
            for kind in [
                OCG_LogTypes::OCG_LOG_TYPE_ERROR as i32,
                OCG_LogTypes::OCG_LOG_TYPE_FROM_SCRIPT as i32,
                OCG_LogTypes::OCG_LOG_TYPE_FOR_DEBUG as i32,
                OCG_LogTypes::OCG_LOG_TYPE_UNDEFINED as i32,
                99,
            ] {
                unsafe {
                    log_handler_trampoline(
                        (&mut context as *mut DuelContext).cast(),
                        message.as_ptr(),
                        kind,
                    )
                };
            }
        });
        let events = events.lock().unwrap();
        assert_eq!(
            events.iter().map(|(level, _)| *level).collect::<Vec<_>>(),
            [
                Level::ERROR,
                Level::DEBUG,
                Level::DEBUG,
                Level::WARN,
                Level::WARN
            ]
        );
        assert!(
            events
                .iter()
                .all(|(_, fields)| fields.message == "core message")
        );
        assert_eq!(events[3].1.log_type, Some(3));
        assert_eq!(events[4].1.log_type, Some(99));
    }

    #[test]
    fn unhandled_message_bytes_are_only_logged_at_trace() {
        let subscriber = RecordingSubscriber::default();
        let events = subscriber.events.clone();
        tracing::subscriber::with_default(subscriber, || {
            crate::ocgcore::messages::parse_messages(&[1, 0, 0, 0, 255]).unwrap();
        });
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, Level::DEBUG);
        assert_eq!(events[0].1.message, "Received unhandled message");
        assert_eq!(events[0].1.bytes, None);
        assert_eq!(events[1].0, Level::TRACE);
        assert_eq!(events[1].1.bytes.as_deref(), Some("[1, 0, 0, 0, 255]"));
    }
}
