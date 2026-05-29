use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use crate::ocgcore::ffi::OCGCoreInstance;

pub struct LogHandler(pub JsValue);

impl LogHandler {
    pub fn new<LogHandlerFn>(instance: OCGCoreInstance, mut log_handler_fn: LogHandlerFn) -> Self
    where
        LogHandlerFn: FnMut(String) + 'static,
    {
        let log_handler = Closure::wrap(Box::new(move |_: u32, string_ptr: u32, _: u32| {
            let buffer = instance.get_wasm_memory().buffer();
            let view = Uint8Array::new_with_byte_offset_and_length(&buffer, string_ptr, 2048);
            let mut length = 0;
            while length < view.length() && view.get_index(length) != 0 {
                length += 1;
            }
            let bytes = view.subarray(0, length).to_vec();
            let message = String::from_utf8_lossy(&bytes);

            log_handler_fn(message.to_string())
        }) as Box<dyn FnMut(u32, u32, u32)>);

        Self(log_handler.into_js_value())
    }
}
