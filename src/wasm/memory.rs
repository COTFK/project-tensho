use super::OCGCORE_MODULE;
use super::get_module_function;
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Manually allocated memory in WASM.
///
/// [`WASMMemoryAllocation::new()`] sets up the _malloc call,
/// and the [`Drop`] implementation handles the _free call automatically.
///
#[derive(Debug, Clone)]
pub struct WASMMemoryAllocation {
    pointer: JsValue,
}

impl WASMMemoryAllocation {
    /// Allocate `length` bytes of memory.
    pub fn new(length: i32) -> anyhow::Result<Self> {
        let malloc = get_module_function("_malloc")?;

        let pointer = malloc
            .call(&JsValue::undefined(), (&JsValue::from_f64(length as f64),))
            .map_err(|e| anyhow::anyhow!("Failed to allocate {length} bytes of memory: {e:#?}"))?;

        Ok(Self { pointer })
    }

    pub fn get_pointer(&self) -> f64 {
        self.pointer.as_f64().unwrap()
    }
}

impl Drop for WASMMemoryAllocation {
    /// Free the allocated memory.
    fn drop(&mut self) {
        tracing::debug!("Freeing pointer {:#?}", self.pointer);
        let free = get_module_function("_free").unwrap();

        let _ = free.call(&JsValue::undefined(), (&self.pointer,));
    }
}

/// Get WASM memory buffer
pub fn get_wasm_memory() -> anyhow::Result<js_sys::WebAssembly::Memory> {
    let module_opt = OCGCORE_MODULE.lock().unwrap();
    let module = module_opt
        .as_ref()
        .ok_or(anyhow::anyhow!("Module not loaded"))?;

    let memory = Reflect::get(module, &"wasmMemory".into())
        .map_err(|e| anyhow::anyhow!("Memory not found: {e:#?}"))?;

    let memory: js_sys::WebAssembly::Memory = memory
        .dyn_into()
        .map_err(|e| anyhow::anyhow!("wasmMemory/memory is not a WebAssembly.Memory: {e:#?}"))?;

    Ok(memory)
}

pub fn get_memory_view() -> anyhow::Result<js_sys::DataView> {
    let memory = get_wasm_memory()?;
    let buffer: js_sys::ArrayBuffer = memory.buffer().into();
    let view = js_sys::DataView::new(&buffer, 0, buffer.byte_length() as usize);

    Ok(view)
}
