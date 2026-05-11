mod ocgcore;

use crate::ocgcore::load_ocgcore;
use dioxus::prelude::*;

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
static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let _ = (OCGCORE_WASM, OCGCORE_JS);
    let ocgcore_status = use_resource(|| async move { load_ocgcore().await });

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div {
            {match ocgcore_status.read().as_ref() {
                None => "loading ocgcore...".to_string(),
                Some(Ok(())) => "ocgcore loaded".to_string(),
                Some(Err(err)) => format!("ocgcore load error: {}", err),
            }}
        }
    }
}
