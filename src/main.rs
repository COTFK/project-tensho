mod ocgcore;
mod settings;
mod state;
mod ui;
mod utility;

use dioxus::prelude::*;

use crate::state::cache_cards;
use crate::state::load_core;
use crate::ui::DuelWrapper;
use crate::ui::LoadingScreen;
use crate::ui::RotateDeviceOverlay;
use crate::utility::BUILD_VERSION;
use crate::utility::GIT_HASH;

static _TAILWIND: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::builder().with_hash_suffix(false)
);

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/?:hand")]
    App { hand: Option<String> },
}

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(desktop!(dioxus::desktop::Config::new().with_custom_head(
            format!("<link rel=\"stylesheet\" href=\"{_TAILWIND}\">")
        )))
        .launch(|| rsx! { Router::<Route> {} });
}

#[component]
pub fn App(hand: Option<String>) -> Element {
    let cache_resource = use_resource(cache_cards);
    if cache_resource.read().is_none() {
        return rsx! {LoadingScreen {  }};
    }

    let core_resource = use_resource(load_core);
    if core_resource.read().is_none() {
        return rsx! {LoadingScreen {  }};
    }

    rsx!(
        RotateDeviceOverlay {}
        DuelWrapper {
            core_resource: core_resource,
            custom_hand: hand,
        }
        div {
            class: "fixed bottom-3 left-3 z-50 rounded-md bg-gray-950/70 px-2 py-1 font-mono text-gray-200 text-[8px] md:text-xs lg:text-base shadow-lg ring-1 ring-white/10 backdrop-blur-sm pointer-events-none",
            title: "Build version and commit hash",
            {format!("tensho v{BUILD_VERSION} · {GIT_HASH}")}
        }
    )
}
