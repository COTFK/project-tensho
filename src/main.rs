#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
)]

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
    let core_resource = use_resource(move || OCGCore::load());

    match &*core_resource.read() {
        Some(result) => match result {
            Ok(core) => {
                use_context_provider(|| core.clone());
                rsx!(App {})
            }
            Err(e) => {
                rsx!("{e:#?}")
            }
        },
        None => {
            rsx!("Loading...")
        }
    }
}
