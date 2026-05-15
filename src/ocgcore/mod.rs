pub mod constants;
mod data;
mod duel;
mod memory;

use anyhow::anyhow;
use dioxus::prelude::*;
use js_sys::Function;
use js_sys::Reflect;
use js_sys::futures::JsFuture;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::ocgcore::memory::CoreMemoryAllocation;
use crate::ocgcore::memory::CorePointer;

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
    #[derive(Debug, Clone)]
    pub type OCGCoreInstance;

    #[wasm_bindgen(js_name = default)]
    fn init_core() -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = _OCG_GetVersion)]
    fn get_version(this: &OCGCoreInstance, major: u32, minor: u32);

    // Emscripten helpers
    #[wasm_bindgen(method, js_name = _malloc)]
    fn malloc(this: &OCGCoreInstance, size: u32) -> u32;

    #[wasm_bindgen(method, js_name = _free)]
    fn free(this: &OCGCoreInstance, ptr: u32);
}

/// Unit struct provider for `ocgcore` functions.
#[derive(Debug, Clone)]
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

    /// Get an ocgcore API (or Emscripten) function as a [`js_sys::Function`].
    pub fn get_function(&self, name: &str) -> anyhow::Result<Function> {
        tracing::debug!("Looking for function: {}", name);

        let func = Reflect::get(&self.0, &name.into())
            .map_err(|e| anyhow::anyhow!("Failed to get property {}: {e:?}", name))?;

        if func.is_undefined() {
            return Err(anyhow::anyhow!("Function {} is undefined on module", name));
        }

        if func.is_null() {
            return Err(anyhow::anyhow!("Function {} is null on module", name));
        }

        let func: Function = func
            .dyn_into()
            .map_err(|_| anyhow::anyhow!("{} exists but is not a function", name))?;

        tracing::debug!("Found function: {}", name);
        Ok(func)
    }

    /// Get the core version as [`(major, minor)`].
    pub fn get_version(&self) -> anyhow::Result<(i32, i32)> {
        // Allocate 8 bytes (two ints) once instead of 4 bytes (one int) twice
        let version_alloc = self.allocate_memory(8)?;
        let major_version_ptr = version_alloc.get_pointer();
        let minor_version_ptr = major_version_ptr.offset_by(4);

        self.0
            .get_version(major_version_ptr.into(), minor_version_ptr.into());

        let view = self.get_memory_view()?;

        // Read sequentially from the single allocation
        let major = view.get_int32_endian(major_version_ptr.into(), true);
        let minor = view.get_int32_endian(minor_version_ptr.into(), true);

        Ok((major, minor))
    }

    pub fn allocate_memory(&self, length: u32) -> anyhow::Result<CoreMemoryAllocation> {
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

    pub fn get_memory_view(&self) -> anyhow::Result<js_sys::DataView> {
        let memory = self.get_wasm_memory()?;
        let buffer: js_sys::ArrayBuffer = memory.buffer().into();
        let view = js_sys::DataView::new(&buffer, 0, buffer.byte_length() as usize);

        Ok(view)
    }
}
