use js_sys::Uint32Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use crate::ocgcore::ffi::OCGCoreInstance;

pub struct CardReaderDone(pub JsValue);

impl CardReaderDone {
    pub fn new(instance: OCGCoreInstance) -> Self {
        let done = Closure::wrap(Box::new(move |_: u32, data_ptr: u32| {
            if data_ptr == 0 {
                return;
            }

            let buffer = instance.get_wasm_memory().buffer();

            // setcodes field is at byte offset 8 in OCG_CardData / OCGCardData
            let ptr_view = Uint32Array::new_with_byte_offset_and_length(&buffer, data_ptr + 8, 1);
            let setcodes_ptr = ptr_view.get_index(0);
            if setcodes_ptr != 0 {
                instance.free(setcodes_ptr);
            }
        }) as Box<dyn FnMut(u32, u32)>);

        Self(done.into_js_value())
    }
}
