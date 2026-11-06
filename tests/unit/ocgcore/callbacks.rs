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
fn unhandled_message_logs_type_and_bytes_at_error() {
    let subscriber = RecordingSubscriber::default();
    let events = subscriber.events.clone();
    tracing::subscriber::with_default(subscriber, || {
        crate::ocgcore::messages::parse_messages(&[1, 0, 0, 0, 255]).unwrap();
    });
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, Level::ERROR);
    assert_eq!(
        events[0].1.message,
        "Received unhandled message 255 with bytes: [1, 0, 0, 0, 255]"
    );
    assert_eq!(events[0].1.bytes, None);
}
