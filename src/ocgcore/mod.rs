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
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::ocgcore::data::OCGCardData;
use crate::ocgcore::data::OCGDuelOptions;
pub use crate::ocgcore::duel::Duel;
use crate::ocgcore::memory::CoreMemoryAllocation;
use crate::ocgcore::memory::CorePointer;
pub use crate::utility::get_cached_script;

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

/// Unit struct provider for `ocgcore` functions.
#[derive(Debug, Clone, PartialEq)]
pub struct OCGCore(OCGCoreInstance);

impl OCGCore {
    pub async fn load() -> anyhow::Result<Self> {
        let promise = init_core();

        let ocgcore = JsFuture::from(promise)
            .await
            .map_err(|e| anyhow!("Core initialization failed: {e:?}"))?;

        let instance: OCGCoreInstance = ocgcore.unchecked_into();

        tracing::debug!("Core loaded successfully.");
        Ok(OCGCore(instance))
    }

    pub fn create_duel(&self) -> anyhow::Result<Duel<'_>> {
        let (card_reader, script_reader, log_handler, card_reader_done) =
            ensure_duel_callbacks(self)?;

        let mut options = OCGDuelOptions::default();
        options.card_reader = card_reader;
        options.script_reader = script_reader;
        options.log_handler = log_handler;
        options.card_reader_done = card_reader_done;

        let options_size = std::mem::size_of::<OCGDuelOptions>();
        let options_alloc = self.allocate_memory(options_size as u32)?;
        let options_ptr = options_alloc.get_pointer();

        let wasm_mem = self.get_wasm_memory()?;

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

        let status = self.0.create_duel(duel_out_ptr.into(), options_ptr.into());
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
        let pointer = self.0.malloc(length);

        Ok(CoreMemoryAllocation::new(self, CorePointer::new(pointer)))
    }

    pub fn get_wasm_memory(&self) -> anyhow::Result<js_sys::WebAssembly::Memory> {
        let memory = Reflect::get(&self.0, &"wasmMemory".into())
            .map_err(|e| anyhow::anyhow!("Memory not found: {e:#?}"))?;

        let memory: js_sys::WebAssembly::Memory = memory.dyn_into().map_err(|e| {
            anyhow::anyhow!("wasmMemory/memory is not a WebAssembly.Memory: {e:#?}")
        })?;

        Ok(memory)
    }
}

thread_local! {
    // Hold JsValue representations of closures so they are not dropped and stay registered with the wasm runtime.
    static CALLBACK_KEEP_ALIVE: RefCell<Vec<JsValue>> = RefCell::new(Vec::new());
    // Store only numeric callback indices returned by `addFunction`.
    static DUEL_CALLBACKS: RefCell<Option<(u32, u32, u32, u32)>> = RefCell::new(None);
}

fn ensure_duel_callbacks(core: &OCGCore) -> anyhow::Result<(u32, u32, u32, u32)> {
    DUEL_CALLBACKS.with(|slot| -> anyhow::Result<(u32, u32, u32, u32)> {
        let mut slot = slot.borrow_mut();

        if let Some(indices) = *slot {
            return Ok(indices);
        }

        // 1. Build closures
        // Use u32 for parameters that represent pointers or IDs in WASM
        let card_reader = Closure::wrap(Box::new(move |_payload: u32, code: u32, data_ptr: u32| {
            if data_ptr == 0 {
                tracing::warn!(target: "ocgcore", "card_reader received null output pointer for code {code}");
                return;
            }

            let data = OCGCardData::with_code(code);
            let data_size = std::mem::size_of::<OCGCardData>();
            let mut data_bytes = [0u8; std::mem::size_of::<OCGCardData>()];
            data.write_bytes(&mut data_bytes);

            let wasm_mem = core.get_wasm_memory().unwrap();
            let dest_view = Uint8Array::new_with_byte_offset_and_length(
                &wasm_mem.buffer(),
                data_ptr,
                data_size as u32,
            );
            dest_view.set(&Uint8Array::from(data_bytes.as_slice()), 0);
        }) as Box<dyn FnMut(u32, u32, u32)>);

        // Clone a reference to core so the closure can use its allocation tools
        let core_clone = core.clone();

        let script_reader = Closure::wrap(Box::new(move |_payload: u32, duel: u32, name_ptr: u32| -> i32 {
            let memory = core_clone.get_wasm_memory().unwrap();

            // 1. Read the requested script name from Emscripten memory
            let script_name = read_c_string(memory, name_ptr.into())
                .unwrap_or_else(|_| "unknown.lua".to_string());

            if script_name == "c0.lua" {
                return 0;
            }

            // 2. Fetch script text via a synchronous cache lookup
            let script_bytes = match get_cached_script(&script_name) {
                Some(bytes) => bytes,
                None => {
                    tracing::error!(target: "ocgcore", "CRITICAL: Cache miss for script '{}'!", script_name);
                    return 0; // Return 0 to indicate failure to ocgcore
                }
            };

            // 3. Allocate space in WASM heap for the script content text
            let content_len = script_bytes.len() as u32;
            let content_alloc = match core_clone.allocate_memory(content_len) {
                Ok(alloc) => alloc,
                Err(e) => {
                    tracing::error!(target: "ocgcore", "Failed to allocate buffer for script content: {e}");
                    return 0;
                }
            };
            let content_ptr = content_alloc.get_pointer();

            // 4. Allocate space in WASM heap for the script name string (including null-terminator)
            let name_bytes = format!("{}\0", script_name).into_bytes();
            let name_len = name_bytes.len() as u32;
            let name_alloc = match core_clone.allocate_memory(name_len) {
                Ok(alloc) => alloc,
                Err(e) => {
                    tracing::error!(target: "ocgcore", "Failed to allocate buffer for script name: {e}");
                    return 0;
                }
            };
            let script_name_ptr = name_alloc.get_pointer();

            // 5. Copy the actual bytes into the allocated WASM memory buffers
            let wasm_mem = core_clone.get_wasm_memory().unwrap();

            let content_view = Uint8Array::new_with_byte_offset_and_length(
                &wasm_mem.buffer(),
                content_ptr.into(),
                content_len,
            );
            content_view.set(&Uint8Array::from(script_bytes.as_slice()), 0);

            let name_view = Uint8Array::new_with_byte_offset_and_length(
                &wasm_mem.buffer(),
                script_name_ptr.into(),
                name_len,
            );
            name_view.set(&Uint8Array::from(name_bytes.as_slice()), 0);

            // 6. Invoke the underlying JS Emscripten function binding
            // Signature: load_script(this, duel, buffer_ptr, len, name_ptr)
            let result = core_clone.0.load_script(
                duel,
                content_ptr.into(),
                content_len,
                script_name_ptr.into()
            );

            // Memory cleanup: CoreMemoryAllocation implements Drop and will free automatically,
            // but ensure they survive until after the function call executes.
            std::mem::drop(content_alloc);
            std::mem::drop(name_alloc);

            // Return 1 for success, 0 for failure back to ocgcore
            if result >= 0 { 1 } else { 0 }
        }) as Box<dyn FnMut(u32, u32, u32) -> i32>);

        let log_handler = Closure::wrap(Box::new(move |_payload: u32, string_ptr: u32, log_type: u32| {
            if string_ptr == 0 {
                info!(target: "ocgcore", "log_handler received null pointer");
                return;
            }

            let memory = core.get_wasm_memory().unwrap();
            let buffer = memory.buffer();

            // 2. Find the null terminator manually in the buffer
            let view = Uint8Array::new_with_byte_offset_and_length(&buffer, string_ptr, 2048); // limit to 2kb
            let mut length = 0;
            while length < view.length() && view.get_index(length) != 0 {
                length += 1;
            }

            // 3. Extract the bytes and convert to string
            let bytes = view.subarray(0, length).to_vec();
            let message = String::from_utf8_lossy(&bytes);

            info!(
                target: "ocgcore",
                type = log_type,
                "{}", message.trim()
            );
        }) as Box<dyn FnMut(u32, u32, u32)>);

        let card_reader_done = Closure::wrap(Box::new(move |payload: u32, data_ptr: u32| {
        }) as Box<dyn FnMut(u32, u32)>);

        // 2. Register functions using the new typed declaration
        // The core.0 is your OCGCoreInstance
        let instance = &core.0;

        let card_reader_index = instance.add_function(card_reader.as_ref().unchecked_ref(), "viii");

        let script_reader_index = instance.add_function(script_reader.as_ref().unchecked_ref(), "iiii");

        let log_handler_index = instance.add_function(log_handler.as_ref().unchecked_ref(), "viii");

        let card_reader_done_index = instance.add_function(card_reader_done.as_ref().unchecked_ref(), "vii");

        // 3. Keep closures alive
        CALLBACK_KEEP_ALIVE.with(|vec| {
            let mut v = vec.borrow_mut();
            v.push(card_reader.into_js_value());
            v.push(script_reader.into_js_value());
            v.push(log_handler.into_js_value());
            v.push(card_reader_done.into_js_value());
        });

        let indices = (
            card_reader_index,
            script_reader_index,
            log_handler_index,
            card_reader_done_index,
        );
        *slot = Some(indices);
        Ok(indices)
    })
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
