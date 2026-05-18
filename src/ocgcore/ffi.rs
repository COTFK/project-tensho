use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "/assets/ocgcore.js")]
extern "C" {
    #[derive(Debug, Clone, PartialEq)]
    pub type OCGCoreInstance;

    #[wasm_bindgen(js_name = default)]
    pub fn init_core() -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = _OCG_GetVersion)]
    pub fn get_version(this: &OCGCoreInstance, major: u32, minor: u32);

    #[wasm_bindgen(method, js_name = _OCG_CreateDuel)]
    pub fn create_duel(this: &OCGCoreInstance, duel: u32, options: u32) -> i32;

    #[wasm_bindgen(method, js_name = _OCG_StartDuel)]
    pub fn start_duel(this: &OCGCoreInstance, duel: u32);

    #[wasm_bindgen(method, js_name = _OCG_DuelProcess)]
    pub fn process(this: &OCGCoreInstance, duel: u32) -> u32;

    #[wasm_bindgen(method, js_name = _OCG_DuelGetMessage)]
    pub fn get_message(this: &OCGCoreInstance, duel: u32, length_ptr: u32) -> u32;

    #[wasm_bindgen(method, js_name = _OCG_DuelSetResponse)]
    pub fn set_response(this: &OCGCoreInstance, duel: u32, response_ptr: u32, len: u32);

    #[wasm_bindgen(method, js_name = _OCG_DuelNewCard)]
    pub fn add_card(this: &OCGCoreInstance, duel: u32, info: u32);

    #[wasm_bindgen(method, js_name = _OCG_LoadScript)]
    pub fn load_script(this: &OCGCoreInstance, duel: u32, buffer: u32, len: u32, name: u32) -> i32;

    #[wasm_bindgen(method, js_name = _OCG_DestroyDuel)]
    pub fn destroy_duel(this: &OCGCoreInstance, duel: u32);

    #[wasm_bindgen(method, js_name = _OCG_DuelQueryCount)]
    pub fn query_count(this: &OCGCoreInstance, duel: u32, team: u8, location: u32) -> u32;

    #[wasm_bindgen(method, js_name = _OCG_DuelQueryLocation)]
    pub fn query_location(this: &OCGCoreInstance, duel: u32, length_ptr: u32, info_ptr: u32)
    -> u32;

    // Emscripten helpers
    #[wasm_bindgen(method, getter, js_name = wasmMemory)]
    pub fn get_wasm_memory(this: &OCGCoreInstance) -> js_sys::WebAssembly::Memory;

    #[wasm_bindgen(method, js_name = _malloc)]
    pub fn malloc(this: &OCGCoreInstance, size: u32) -> u32;

    #[wasm_bindgen(method, js_name = _free)]
    pub fn free(this: &OCGCoreInstance, ptr: u32);

    #[wasm_bindgen(method, js_name = addFunction)]
    pub fn add_function(this: &OCGCoreInstance, func: &js_sys::Function, signature: &str) -> u32;
}
