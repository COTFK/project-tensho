mod memory;
use anyhow::Context;
pub use memory::WASMMemoryAllocation;
pub use memory::get_memory_view;
pub use memory::get_wasm_memory;

use js_sys::{Function, Object, Reflect};
use lazy_static::lazy_static;
use std::sync::Mutex;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

lazy_static! {
    static ref OCGCORE_MODULE: Mutex<Option<Object>> = Mutex::new(None);
}

/// Load the Emscripten ocgcore module dynamically
pub async fn load_ocgcore() -> Result<(), String> {
    tracing::info!("Starting WASM module load");

    // Create an import function: new Function("u", "return import(u);")
    let import_fn = Function::new_with_args("u", "return import(u);");

    // Call import("/assets/ocgcore.js")
    tracing::debug!("Importing /assets/ocgcore.js");
    let promise = import_fn
        .call1(&JsValue::undefined(), &"/assets/ocgcore.js".into())
        .map_err(|e| format!("Import failed: {e:?}"))?;

    // Convert to Promise
    let promise: js_sys::Promise = promise
        .dyn_into()
        .map_err(|_| "Import did not return a promise".to_string())?;

    // Wait for module to load
    tracing::debug!("Waiting for promise to resolve");
    let module = JsFuture::from(promise)
        .await
        .map_err(|e| format!("Import await failed: {e:?}"))?;

    tracing::debug!("Module imported, extracting default export");

    // Get the default export (Emscripten module)
    let default_export = Reflect::get(&module, &"default".into())
        .map_err(|_| "Missing default export".to_string())?;

    tracing::debug!("Got default export, checking if it's callable");

    // The default export should be a function that initializes the module
    let init_fn: Function = default_export
        .dyn_into()
        .map_err(|_| "Default export is not callable".to_string())?;

    // Call the init function
    tracing::debug!("Calling init function");
    let init_result = init_fn
        .call0(&JsValue::undefined())
        .map_err(|e| format!("Module init call failed: {e:?}"))?;

    // Check if it returns a Promise (async init)
    tracing::debug!("Checking if init result is a promise");
    let ocgcore_obj = if let Ok(promise) = init_result.clone().dyn_into::<js_sys::Promise>() {
        tracing::debug!("Init result is a promise, waiting for resolution");
        JsFuture::from(promise)
            .await
            .map_err(|e| format!("Init promise await failed: {e:?}"))?
    } else {
        tracing::debug!("Init result is synchronous");
        init_result
    };

    // Verify it's an object
    let ocgcore_obj: Object = ocgcore_obj
        .dyn_into()
        .map_err(|_| "Module is not an object".to_string())?;

    tracing::info!("Module loaded, enumerating exports");

    // Debug: log what keys are available
    let keys = js_sys::Object::keys(&ocgcore_obj);
    let mut key_list = Vec::new();
    for i in 0..keys.length() {
        if let Some(key) = keys.get(i).as_string() {
            key_list.push(key.clone());
            tracing::info!("WASM module export: {}", key);
        }
    }

    if key_list.is_empty() {
        tracing::warn!("No exports found in WASM module!");
    }

    // Store the module
    *OCGCORE_MODULE.lock().unwrap() = Some(ocgcore_obj);
    tracing::info!("WASM module stored successfully");
    Ok(())
}

/// Get a function from the module by name
pub fn get_module_function(name: &str) -> anyhow::Result<Function> {
    let lock = OCGCORE_MODULE
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to acquire lock: Mutex is poisoned"))?;

    let module = lock.as_ref().with_context(|| "Module not loaded")?;

    tracing::debug!("Looking for function: {}", name);

    let func = Reflect::get(module, &name.into())
        .map_err(|e| anyhow::anyhow!("Failed to get property {}: {e:?}", name))?;

    if func.is_undefined() {
        anyhow::anyhow!("Function {} is undefined on module", name);
    }

    if func.is_null() {
        anyhow::anyhow!("Function {} is null on module", name);
    }

    let func: Function = func
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("{} exists but is not a function", name))?;

    tracing::debug!("Found function: {}", name);
    Ok(func)
}
