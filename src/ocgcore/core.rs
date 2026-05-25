use anyhow::anyhow;
use dioxus::prelude::*;
use js_sys::Uint8Array;
use js_sys::Uint32Array;
use js_sys::futures::JsFuture;
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use super::data::OCGCardData;
use super::ffi::{OCGCoreInstance, init_core};
use crate::ocgcore::data::OCGDuelOptions;
pub use crate::ocgcore::duel::Duel;
use crate::ocgcore::memory::CoreMemoryAllocation;
use crate::ocgcore::memory::CorePointer;
use crate::utility::STATIC_CARD_DATA;
pub use crate::utility::get_cached_script;

/// Wraps `OCGCore` WASM instance with persistent callback state.
#[derive(Debug, Clone, PartialEq)]
pub struct OCGCore {
    pub(super) instance: OCGCoreInstance,
    callback_indices: (u32, u32, u32),
    // Keep callback JsValue refs alive for entire OCGCore lifetime
    _callback_refs: std::sync::Arc<Vec<JsValue>>,
}

impl OCGCore {
    pub async fn load() -> anyhow::Result<Self> {
        let module = js_sys::Object::new();
        let locate_file = Closure::wrap(Box::new(|path: JsValue, _prefix: JsValue| -> JsValue {
            let file_name = path.as_string().unwrap_or_default();

            if file_name == "ocgcore.wasm" {
                JsValue::from_str("/ocgcore.wasm")
            } else {
                JsValue::from_str(&format!("/{file_name}"))
            }
        }) as Box<dyn FnMut(JsValue, JsValue) -> JsValue>);

        Reflect::set(&module, &JsValue::from_str("locateFile"), locate_file.as_ref())
            .map_err(|e| anyhow!("Failed to configure ocgcore locateFile: {e:?}"))?;

        let promise = init_core(&module.into());

        let ocgcore = JsFuture::from(promise)
            .await
            .map_err(|e| anyhow!("Core initialization failed: {e:?}"))?;

        drop(locate_file);

        let instance: OCGCoreInstance = ocgcore.unchecked_into();

        // Build callbacks once at init
        let (callback_indices, callback_refs) = Self::build_callbacks(&instance)?;

        tracing::debug!("Core loaded successfully.");
        Ok(Self {
            instance,
            callback_indices,
            _callback_refs: std::sync::Arc::new(callback_refs),
        })
    }

    pub fn create_duel(&self) -> anyhow::Result<Duel> {
        let mut options = OCGDuelOptions::default();
        options.card_reader = self.callback_indices.0;
        options.script_reader = self.callback_indices.1;
        options.log_handler = self.callback_indices.2;

        let options_size = std::mem::size_of::<OCGDuelOptions>();
        let options_alloc = self.allocate_memory(options_size as u32);
        let options_ptr = options_alloc.get_pointer();

        let wasm_mem = self.get_wasm_memory();

        // Perform a bulk memory copy (much faster than a loop)
        let options_bytes =
            unsafe { std::slice::from_raw_parts((&raw const options).cast::<u8>(), options_size) };
        let dest_view = Uint8Array::new_with_byte_offset_and_length(
            &wasm_mem.buffer(),
            options_ptr.into(),
            options_size as u32,
        );
        dest_view.set(&Uint8Array::from(options_bytes), 0);

        // Allocate space for the returned Duel pointer (4 bytes for WASM32)
        let duel_out_alloc = self.allocate_memory(4);
        let duel_out_ptr = duel_out_alloc.get_pointer();

        let status = self
            .instance
            .create_duel(duel_out_ptr.into(), options_ptr.into());
        if status != 0 {
            return Err(anyhow!("_OCG_CreateDuel failed with status {status}"));
        }

        // Extract the actual handle written by ocgcore into our allocated space
        let handle = Uint32Array::new_with_byte_offset_and_length(
            &wasm_mem.buffer(),
            duel_out_ptr.into(),
            1,
        )
        .get_index(0);

        Ok(Duel::new(CorePointer(handle), self))
    }

    pub fn allocate_memory(&self, length: u32) -> CoreMemoryAllocation<'_> {
        CoreMemoryAllocation::new(self, CorePointer::new(self.instance.malloc(length)))
    }

    pub fn get_wasm_memory(&self) -> js_sys::WebAssembly::Memory {
        self.instance.get_wasm_memory()
    }

    fn build_callbacks(
        instance: &OCGCoreInstance,
    ) -> anyhow::Result<((u32, u32, u32), Vec<JsValue>)> {
        let mut callback_refs = Vec::new();
        let instance = instance.clone();

        // Card reader callback
        let inst = instance.clone();
        let card_reader = Closure::wrap(Box::new(move |_: u32, code: u32, data_ptr: u32| {
            if data_ptr == 0 {
                return;
            }
            let mut data = STATIC_CARD_DATA
                .iter()
                .find(|(id, _)| *id == code)
                .map(|(_, card_data)| card_data)
                .copied()
                .unwrap_or_else(|| OCGCardData::with_code(code));

            let buffer = inst.get_wasm_memory().buffer();

            // Allocate memory for setcodes array (u16 values + terminator)
            // Format: [setcode1: u16][setcode2: u16]...[0x0000: u16 terminator]
            // For now we handle single setcode values - convert to array
            let setcodes_array = if data.setcodes > 0 {
                // Allocate 4 bytes: 2 bytes for the setcode value + 2 bytes for terminator (0x0000)
                let setcodes_ptr = inst.malloc(4);

                // Write setcode as u16 at offset 0
                let setcode_u16 = (data.setcodes & 0xFFFF) as u16;
                let setcode_bytes = setcode_u16.to_le_bytes();
                let sc_view = Uint8Array::new_with_byte_offset_and_length(&buffer, setcodes_ptr, 2);
                sc_view.set(&Uint8Array::from(setcode_bytes.as_slice()), 0);

                // Write terminator (0x0000) at offset 2
                let term_view =
                    Uint8Array::new_with_byte_offset_and_length(&buffer, setcodes_ptr + 2, 2);
                term_view.set(&Uint8Array::from(&[0u8, 0u8][..]), 0);

                setcodes_ptr
            } else {
                0
            };

            // Update the setcodes field to point to the allocated array
            data.setcodes = setcodes_array;

            // Write OCGCardData to WASM memory
            let mut data_bytes = [0u8; std::mem::size_of::<OCGCardData>()];
            data.write_bytes(&mut data_bytes);

            Uint8Array::new_with_byte_offset_and_length(
                &buffer,
                data_ptr,
                std::mem::size_of::<OCGCardData>() as u32,
            )
            .set(&Uint8Array::from(data_bytes.as_slice()), 0);
        }) as Box<dyn FnMut(u32, u32, u32)>);
        callback_refs.push(card_reader.into_js_value());
        let card_reader_index =
            instance.add_function(callback_refs.last().unwrap().unchecked_ref(), "viii");

        // Script reader callback
        let inst = instance.clone();
        let script_reader =
            Closure::wrap(Box::new(move |_: u32, duel: u32, name_ptr: u32| -> i32 {
                let buffer = inst.get_wasm_memory().buffer();
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

                let script_bytes = match get_cached_script(&script_name) {
                    Some(b) => b,
                    None => return 0,
                };

                let content_len = script_bytes.len() as u32;
                let content_ptr = inst.malloc(content_len);
                if content_ptr == 0 {
                    return 0;
                }

                let name_bytes = format!("{script_name}\0").into_bytes();
                let name_len = name_bytes.len() as u32;
                let script_name_ptr = inst.malloc(name_len);
                if script_name_ptr == 0 {
                    inst.free(content_ptr);
                    return 0;
                }

                Uint8Array::new_with_byte_offset_and_length(&buffer, content_ptr, content_len)
                    .set(&Uint8Array::from(script_bytes.as_slice()), 0);
                Uint8Array::new_with_byte_offset_and_length(&buffer, script_name_ptr, name_len)
                    .set(&Uint8Array::from(name_bytes.as_slice()), 0);

                let result = inst.load_script(duel, content_ptr, content_len, script_name_ptr);
                inst.free(content_ptr);
                inst.free(script_name_ptr);

                i32::from(result >= 0)
            }) as Box<dyn FnMut(u32, u32, u32) -> i32>);
        callback_refs.push(script_reader.into_js_value());
        let script_reader_index =
            instance.add_function(callback_refs.last().unwrap().unchecked_ref(), "iiii");

        // Log handler callback
        let inst = instance.clone();
        let log_handler = Closure::wrap(Box::new(move |_: u32, string_ptr: u32, log_type: u32| {
            let buffer = inst.get_wasm_memory().buffer();
            let view = Uint8Array::new_with_byte_offset_and_length(&buffer, string_ptr, 2048);
            let mut length = 0;
            while length < view.length() && view.get_index(length) != 0 {
                length += 1;
            }
            let bytes = view.subarray(0, length).to_vec();
            let message = String::from_utf8_lossy(&bytes);
            info!(target: "ocgcore", type = log_type, "{}", message.trim());
        }) as Box<dyn FnMut(u32, u32, u32)>);
        callback_refs.push(log_handler.into_js_value());
        let log_handler_index =
            instance.add_function(callback_refs.last().unwrap().unchecked_ref(), "viii");

        Ok((
            (card_reader_index, script_reader_index, log_handler_index),
            callback_refs,
        ))
    }
}
