use anyhow::anyhow;
use js_sys::Reflect;
use js_sys::Uint8Array;
use js_sys::Uint32Array;
use js_sys::futures::JsFuture;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use super::callbacks::CoreCallbacks;
use super::ffi::{OCGCoreInstance, init_core};
use crate::ocgcore::OCGCardData;
use crate::ocgcore::callbacks::CallbackHandles;
use crate::ocgcore::data::OCGDuelOptions;
pub use crate::ocgcore::duel::Duel;
use crate::ocgcore::memory::CoreMemoryAllocation;
use crate::ocgcore::memory::CorePointer;

/// Wraps `OCGCore` WASM instance with persistent callback state.
#[derive(Debug, Clone, PartialEq)]
pub struct OCGCore {
    pub(super) instance: OCGCoreInstance,
    callbacks: CallbackHandles,
}

impl OCGCore {
    pub async fn load<CardReaderFn, ScriptReaderFn, LogHandlerFn>(
        callbacks: CoreCallbacks<CardReaderFn, ScriptReaderFn, LogHandlerFn>,
    ) -> anyhow::Result<Self>
    where
        CardReaderFn: FnMut(u32) -> OCGCardData + 'static,
        ScriptReaderFn: FnMut(&str) -> Option<Vec<u8>> + 'static,
        LogHandlerFn: FnMut(String) + 'static,
    {
        let module = js_sys::Object::new();
        let locate_file = Closure::wrap(Box::new(|path: JsValue, _prefix: JsValue| -> JsValue {
            let file_name = path.as_string().unwrap_or_default();

            if file_name == "ocgcore.wasm" {
                JsValue::from_str("/ocgcore.wasm")
            } else {
                JsValue::from_str(&format!("/{file_name}"))
            }
        }) as Box<dyn FnMut(JsValue, JsValue) -> JsValue>);

        Reflect::set(
            &module,
            &JsValue::from_str("locateFile"),
            locate_file.as_ref(),
        )
        .map_err(|e| anyhow!("Failed to configure ocgcore locateFile: {e:?}"))?;

        let promise = init_core(&module.into());

        let ocgcore = JsFuture::from(promise)
            .await
            .map_err(|e| anyhow!("Core initialization failed: {e:?}"))?;

        drop(locate_file);

        let instance: OCGCoreInstance = ocgcore.unchecked_into();

        let callbacks = callbacks.register(instance.clone());

        Ok(Self {
            instance,
            callbacks,
        })
    }

    pub fn create_duel(&self) -> anyhow::Result<Duel> {
        let mut options = OCGDuelOptions::default();
        options.card_reader = self.callbacks.card_reader;
        options.script_reader = self.callbacks.script_reader;
        options.log_handler = self.callbacks.log_handler;
        options.card_reader_done = self.callbacks.card_reader_done;

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
}
