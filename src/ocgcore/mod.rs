pub mod constants;
mod data;
mod duel;
mod memory;

use anyhow::anyhow;
use dioxus::prelude::*;
use js_sys::Reflect;
use js_sys::Uint8Array;
use js_sys::Uint32Array;
use js_sys::WebAssembly::Memory;
use js_sys::futures::JsFuture;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

pub use crate::ocgcore::data::OCGCardData;
use crate::ocgcore::data::OCGDuelOptions;
pub use crate::ocgcore::duel::Duel;
use crate::ocgcore::memory::CoreMemoryAllocation;
use crate::ocgcore::memory::CorePointer;
pub use crate::utility::get_cached_script;
use crate::utility::STATIC_CARD_DATA;
pub use crate::ocgcore::duel::IdleCommandPayload;

static OCGCORE_WASM: Asset = asset!(
    "/assets/ocgcore.wasm",
    AssetOptions::builder()
        .with_hash_suffix(false)
        .into_asset_options()
);
static OCGCORE_JS: Asset = asset!(
    "/assets/ocgcore.js",
    AssetOptions::js().with_hash_suffix(false)
);

#[wasm_bindgen(raw_module = "/assets/ocgcore.js")]
extern "C" {
    #[derive(Debug, Clone, PartialEq)]
    pub type OCGCoreInstance;

    #[wasm_bindgen(js_name = default)]
    fn init_core() -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = _OCG_GetVersion)]
    fn get_version(this: &OCGCoreInstance, major: u32, minor: u32);

    #[wasm_bindgen(method, js_name = _OCG_CreateDuel)]
    fn create_duel(this: &OCGCoreInstance, duel: u32, options: u32) -> i32;

    #[wasm_bindgen(method, js_name = _OCG_StartDuel)]
    fn start_duel(this: &OCGCoreInstance, duel: u32);

    #[wasm_bindgen(method, js_name = _OCG_DuelProcess)]
    fn process(this: &OCGCoreInstance, duel: u32) -> u32;

    #[wasm_bindgen(method, js_name = _OCG_DuelGetMessage)]
    fn get_message(this: &OCGCoreInstance, duel: u32, length_ptr: u32) -> u32;

    #[wasm_bindgen(method, js_name = _OCG_DuelSetResponse)]
    fn set_response(this: &OCGCoreInstance, duel: u32, response_ptr: u32);

    #[wasm_bindgen(method, js_name = _OCG_DuelNewCard)]
    fn add_card(this: &OCGCoreInstance, duel: u32, info: u32);

    #[wasm_bindgen(method, js_name = _OCG_LoadScript)]
    fn load_script(this: &OCGCoreInstance, duel: u32, buffer: u32, len: u32, name: u32) -> i32;

    #[wasm_bindgen(method, js_name = _OCG_DestroyDuel)]
    fn destroy_duel(this: &OCGCoreInstance, duel: u32);

    #[wasm_bindgen(method, js_name = _OCG_DuelQueryCount)]
    fn query_count(this: &OCGCoreInstance, duel: u32, team: u8, location: u32) -> u32;

    #[wasm_bindgen(method, js_name = _OCG_DuelQueryLocation, catch)]
    fn query_location(
        this: &OCGCoreInstance,
        duel: u32,
        length_ptr: u32,
        info_ptr: u32,
    ) -> Result<u32, JsValue>;

    // Emscripten helpers
    #[wasm_bindgen(method, js_name = _malloc)]
    fn malloc(this: &OCGCoreInstance, size: u32) -> u32;

    #[wasm_bindgen(method, js_name = _free)]
    fn free(this: &OCGCoreInstance, ptr: u32);

    #[wasm_bindgen(method, js_name = addFunction)]
    fn add_function(this: &OCGCoreInstance, func: &js_sys::Function, signature: &str) -> u32;
}

/// Wraps OCGCore WASM instance with persistent callback state.
#[derive(Debug, Clone)]
pub struct OCGCore {
    instance: OCGCoreInstance,
    callback_indices: (u32, u32, u32),
    // Keep callback JsValue refs alive for entire OCGCore lifetime
    _callback_refs: std::sync::Arc<Vec<JsValue>>,
}

impl OCGCore {
    pub async fn load() -> anyhow::Result<Self> {
        let promise = init_core();

        let ocgcore = JsFuture::from(promise)
            .await
            .map_err(|e| anyhow!("Core initialization failed: {e:?}"))?;

        let instance: OCGCoreInstance = ocgcore.unchecked_into();

        // Build callbacks once at init
        let (callback_indices, callback_refs) = Self::build_callbacks(&instance)?;

        tracing::debug!("Core loaded successfully.");
        Ok(OCGCore {
            instance,
            callback_indices,
            _callback_refs: std::sync::Arc::new(callback_refs),
        })
    }

    pub fn create_duel(&self) -> anyhow::Result<Duel<'_>> {
        let mut options = OCGDuelOptions::default();
        options.card_reader = self.callback_indices.0;
        options.script_reader = self.callback_indices.1;
        options.log_handler = self.callback_indices.2;

        let options_size = std::mem::size_of::<OCGDuelOptions>();
        let options_alloc = self.allocate_memory(options_size as u32)?;
        let options_ptr = options_alloc.get_pointer();

        let wasm_mem = self.get_wasm_memory()
            .ok_or_else(|| anyhow!("Failed to get WASM memory"))?;

        // Perform a bulk memory copy (much faster than a loop)
        let options_bytes = unsafe {
            std::slice::from_raw_parts(
                (&options as *const OCGDuelOptions) as *const u8,
                options_size,
            )
        };
        let dest_view = Uint8Array::new_with_byte_offset_and_length(
            &wasm_mem.buffer(),
            options_ptr.into(),
            options_size as u32,
        );
        dest_view.set(&Uint8Array::from(options_bytes), 0);

        // Allocate space for the returned Duel pointer (4 bytes for WASM32)
        let duel_out_alloc = self.allocate_memory(4)?;
        let duel_out_ptr = duel_out_alloc.get_pointer();

        let status = self.instance.create_duel(duel_out_ptr.into(), options_ptr.into());
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

    pub fn allocate_memory(&self, length: u32) -> anyhow::Result<CoreMemoryAllocation<'_>> {
        let pointer = self.instance.malloc(length);

        Ok(CoreMemoryAllocation::new(self, CorePointer::new(pointer)))
    }

    fn get_wasm_memory(&self) -> Option<js_sys::WebAssembly::Memory> {
        get_wasm_memory(&self.instance)
    }

    fn build_callbacks(instance: &OCGCoreInstance) -> anyhow::Result<((u32, u32, u32), Vec<JsValue>)> {
        let mut callback_refs = Vec::new();
        let instance = instance.clone();

        // Card reader callback
        let inst = instance.clone();
        let card_reader = Closure::wrap(Box::new(move |_: u32, code: u32, data_ptr: u32| {
            if data_ptr == 0 {
                return;
            }
            let data = STATIC_CARD_DATA
                .iter()
                .find(|(id, _)| *id == code)
                .map(|(_, card_data)| card_data)
                .copied()
                .unwrap_or_else(|| OCGCardData::with_code(code));
            let mut data_bytes = [0u8; std::mem::size_of::<OCGCardData>()];
            data.write_bytes(&mut data_bytes);
            
            if let Some(memory) = get_wasm_memory(&inst) {
                let view = Uint8Array::new_with_byte_offset_and_length(
                    &memory.buffer(),
                    data_ptr,
                    std::mem::size_of::<OCGCardData>() as u32,
                );
                view.set(&Uint8Array::from(data_bytes.as_slice()), 0);
            }
        }) as Box<dyn FnMut(u32, u32, u32)>);
        callback_refs.push(card_reader.into_js_value());
        let card_reader_index = instance.add_function(
            callback_refs.last().unwrap().unchecked_ref(),
            "viii",
        );

        // Script reader callback
        let inst = instance.clone();
        let script_reader = Closure::wrap(Box::new(move |_: u32, duel: u32, name_ptr: u32| -> i32 {
            let memory = match get_wasm_memory(&inst) {
                Some(m) => m,
                None => return 0,
            };

            let script_name = read_c_string(memory.clone(), name_ptr.into())
                .unwrap_or_else(|_| "unknown.lua".to_string());

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

            let name_bytes = format!("{}\0", script_name).into_bytes();
            let name_len = name_bytes.len() as u32;
            let script_name_ptr = inst.malloc(name_len);
            if script_name_ptr == 0 {
                inst.free(content_ptr);
                return 0;
            }

            write_to_memory(&inst, content_ptr, &script_bytes);
            write_to_memory(&inst, script_name_ptr, &name_bytes);

            let result = inst.load_script(duel, content_ptr, content_len, script_name_ptr);
            inst.free(content_ptr);
            inst.free(script_name_ptr);

            if result >= 0 { 1 } else { 0 }
        }) as Box<dyn FnMut(u32, u32, u32) -> i32>);
        callback_refs.push(script_reader.into_js_value());
        let script_reader_index = instance.add_function(
            callback_refs.last().unwrap().unchecked_ref(),
            "iiii",
        );

        // Log handler callback
        let inst = instance.clone();
        let log_handler = Closure::wrap(Box::new(move |_: u32, string_ptr: u32, log_type: u32| {
            if let Some(memory) = get_wasm_memory(&inst) {
                let buffer = memory.buffer();
                let view = Uint8Array::new_with_byte_offset_and_length(&buffer, string_ptr, 2048);
                let mut length = 0;
                while length < view.length() && view.get_index(length) != 0 {
                    length += 1;
                }
                let bytes = view.subarray(0, length).to_vec();
                let message = String::from_utf8_lossy(&bytes);
                info!(target: "ocgcore", type = log_type, "{}", message.trim());
            }
        }) as Box<dyn FnMut(u32, u32, u32)>);
        callback_refs.push(log_handler.into_js_value());
        let log_handler_index = instance.add_function(
            callback_refs.last().unwrap().unchecked_ref(),
            "viii",
        );

        Ok((
            (card_reader_index, script_reader_index, log_handler_index),
            callback_refs,
        ))
    }
}

fn get_wasm_memory(instance: &OCGCoreInstance) -> Option<js_sys::WebAssembly::Memory> {
    Reflect::get(instance, &"wasmMemory".into())
        .ok()?
        .dyn_into::<js_sys::WebAssembly::Memory>()
        .ok()
}

fn write_to_memory(instance: &OCGCoreInstance, ptr: u32, data: &[u8]) {
    if let Some(memory) = get_wasm_memory(instance) {
        let view = Uint8Array::new_with_byte_offset_and_length(
            &memory.buffer(),
            ptr,
            data.len() as u32,
        );
        view.set(&Uint8Array::from(data), 0);
    }
}

fn read_c_string(memory: Memory, ptr: f64) -> anyhow::Result<String> {
    let buffer = memory.buffer();
    let bytes = Uint8Array::new(&buffer);
    let mut offset = ptr as u32;
    let mut out = Vec::new();

    while offset < bytes.length() {
        let byte = bytes.get_index(offset);
        if byte == 0 {
            break;
        }

        out.push(byte);
        offset += 1;
    }

    String::from_utf8(out).map_err(|e| anyhow!("Invalid UTF-8 in script name: {e}"))
}
