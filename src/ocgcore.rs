use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Window;

/// Load the ocgcore WASM module.
///
/// Web-only: loads the Emscripten JS glue module.
pub async fn load_ocgcore() -> Result<(), String> {
    let window = web_sys::window().ok_or("missing window")?;
    import_emscripten_module(&window, "/assets/ocgcore.js").await?;
    Ok(())
}

async fn import_emscripten_module(_window: &Window, url: &str) -> Result<(), String> {
    let import_fn = js_sys::Function::new_with_args("u", "return import(u);");
    let promise = import_fn
        .call1(&js_sys::global(), &url.into())
        .map_err(|err| format!("dynamic import failed: {err:?}"))?;
    let promise: js_sys::Promise = promise
        .dyn_into()
        .map_err(|_| "import did not return a promise")?;
    let module = JsFuture::from(promise)
        .await
        .map_err(|err| format!("import await failed: {err:?}"))?;

    let default_export = js_sys::Reflect::get(&module, &"default".into())
        .map_err(|_| "missing default export")?;
    let init_fn: js_sys::Function = default_export
        .dyn_into()
        .map_err(|_| "default export is not a function")?;
    let init_promise = init_fn
        .call0(&js_sys::global())
        .map_err(|err| format!("module init failed: {err:?}"))?;
    let init_promise: js_sys::Promise = init_promise
        .dyn_into()
        .map_err(|_| "init did not return a promise")?;
    JsFuture::from(init_promise)
        .await
        .map_err(|err| format!("init await failed: {err:?}"))?;
    Ok(())
}
