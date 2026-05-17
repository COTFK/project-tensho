mod app;
mod ocgcore;
mod ui;
mod utility;

use dioxus::prelude::*;

use crate::app::App;
use crate::ocgcore::OCGCore;

static _OCGCORE_WASM: Asset = asset!(
    "/assets/ocgcore.wasm",
    AssetOptions::builder()
        .with_hash_suffix(false)
        .into_asset_options()
);
static _OCGCORE_JS: Asset = asset!(
    "/assets/ocgcore.js",
    AssetOptions::js().with_hash_suffix(false)
);

fn main() {
    dioxus::launch(AppContainer);
}

#[component]
pub fn AppContainer() -> Element {
    // Load and initialize core
    let core_resource = use_resource(OCGCore::load);

    match &*core_resource.read() {
        Some(Ok(core)) => rsx!(App { core: core.clone() }),
        Some(Err(e)) => rsx!("{e:#?}"),
        None => {
            rsx!("Loading...")
        }
    }
}
