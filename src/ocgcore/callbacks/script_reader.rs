use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use ocgcore_ffi::OCGCore;

pub struct ScriptReader(pub JsValue);

impl ScriptReader {
    pub fn new<ScriptReaderFn>(
        instance: OCGCore,
        mut script_reader_fn: ScriptReaderFn,
    ) -> Self
    where
        ScriptReaderFn: FnMut(&str) -> Option<Vec<u8>> + 'static,
    {
        let script_reader =
            Closure::wrap(Box::new(move |_: u32, duel: u32, name_ptr: u32| -> i32 {
                let buffer = instance.get_wasm_memory().buffer();
                let bytes = Uint8Array::new(&buffer);
                let mut offset = name_ptr;
                let mut out = Vec::new();

                while offset < bytes.length() && bytes.get_index(offset) != 0 {
                    out.push(bytes.get_index(offset));
                    offset += 1;
                }

                let script_name =
                    String::from_utf8(out).unwrap_or_else(|_| "unknown.lua".to_string());

                if script_name == "c0.lua" {
                    return 0;
                }

                let script_bytes = match script_reader_fn(&script_name) {
                    Some(b) => b,
                    None => return 0,
                };

                let content_len = script_bytes.len() as u32;
                let content_ptr = instance.malloc(content_len);
                if content_ptr == 0 {
                    return 0;
                }

                let name_bytes = format!("{script_name}\0").into_bytes();
                let name_len = name_bytes.len() as u32;
                let script_name_ptr = instance.malloc(name_len);
                if script_name_ptr == 0 {
                    instance.free(content_ptr);
                    return 0;
                }

                Uint8Array::new_with_byte_offset_and_length(&buffer, content_ptr, content_len)
                    .set(&Uint8Array::from(script_bytes.as_slice()), 0);
                Uint8Array::new_with_byte_offset_and_length(&buffer, script_name_ptr, name_len)
                    .set(&Uint8Array::from(name_bytes.as_slice()), 0);

                let result = instance.load_script(duel, content_ptr, content_len, script_name_ptr);
                instance.free(content_ptr);
                instance.free(script_name_ptr);

                i32::from(result >= 0)
            }) as Box<dyn FnMut(u32, u32, u32) -> i32>);

        Self(script_reader.into_js_value())
    }
}
