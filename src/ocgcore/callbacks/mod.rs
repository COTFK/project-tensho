mod card_reader;
mod card_reader_done;
mod log_handler;
mod script_reader;

use crate::ocgcore::OCGCardData;
use ocgcore_ffi::OCGCore;
use card_reader::CardReader;
use card_reader_done::CardReaderDone;
use log_handler::LogHandler;
use script_reader::ScriptReader;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, PartialEq)]
pub struct CallbackHandles {
    pub card_reader: u32,
    pub script_reader: u32,
    pub log_handler: u32,
    pub card_reader_done: u32,
    _keepalive: Vec<JsValue>,
}

pub struct CoreCallbacks<CardReaderFn, ScriptReaderFn, LogHandlerFn> {
    pub card_reader: CardReaderFn,
    pub script_reader: ScriptReaderFn,
    pub log_handler: LogHandlerFn,
}

impl<CardReaderFn, ScriptReaderFn, LogHandlerFn>
    CoreCallbacks<CardReaderFn, ScriptReaderFn, LogHandlerFn>
{
    pub fn new(
        card_reader: CardReaderFn,
        script_reader: ScriptReaderFn,
        log_handler: LogHandlerFn,
    ) -> Self
    where
        CardReaderFn: FnMut(u32) -> OCGCardData + 'static,
        ScriptReaderFn: FnMut(&str) -> Option<Vec<u8>> + 'static,
        LogHandlerFn: FnMut(String) + 'static,
    {
        Self {
            card_reader,
            script_reader,
            log_handler,
        }
    }

    pub fn register(self, instance: OCGCore) -> CallbackHandles
    where
        CardReaderFn: FnMut(u32) -> OCGCardData + 'static,
        ScriptReaderFn: FnMut(&str) -> Option<Vec<u8>> + 'static,
        LogHandlerFn: FnMut(String) + 'static,
    {
        let card_reader = CardReader::new(instance.clone(), self.card_reader);
        let script_reader = ScriptReader::new(instance.clone(), self.script_reader);
        let log_handler = LogHandler::new(instance.clone(), self.log_handler);
        let card_reader_done = CardReaderDone::new(instance.clone());

        let keepalive = vec![
            card_reader.0,
            script_reader.0,
            log_handler.0,
            card_reader_done.0,
        ];

        CallbackHandles {
            card_reader: instance.add_function(keepalive[0].unchecked_ref(), "viii"),
            script_reader: instance.add_function(keepalive[1].unchecked_ref(), "iiii"),
            log_handler: instance.add_function(keepalive[2].unchecked_ref(), "viii"),
            card_reader_done: instance.add_function(keepalive[3].unchecked_ref(), "vii"),
            _keepalive: keepalive,
        }
    }
}
