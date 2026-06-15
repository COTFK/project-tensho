mod ocgcore;
mod state;
mod ui;
mod utility;
mod settings;

use dioxus::prelude::*;

use crate::state::cache_dependencies;
use crate::state::load_duel;
use crate::ui::DuelScreen;
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
    AppContainer { hand: Option<String> },
}


fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(desktop!(dioxus::desktop::Config::new().with_custom_head(
            format!("<link rel=\"stylesheet\" href=\"{_TAILWIND}\">")
        )))
        .launch(|| rsx! { Router::<Route> {} });
}

#[component]
pub fn AppContainer(hand: Option<String>) -> Element {
    let cache_resource = use_resource(cache_dependencies);

    let core_resource = use_resource(move || load_duel(cache_resource, hand.clone()));

    rsx!(
        RotateDeviceOverlay {}
        match &*core_resource.read() {
            Some(Ok(duel)) => rsx!(
                DuelScreen {
                    duel: duel.clone(),
                    resource_handle: core_resource,
                }
            ),
            Some(Err(e)) => rsx!("{e:#?}"),
            None => rsx!(LoadingScreen {}),
        }
        div {
            class: "fixed bottom-3 left-3 z-50 rounded-md bg-gray-950/70 px-2 py-1 font-mono text-gray-200 text-[8px] md:text-xs lg:text-base shadow-lg ring-1 ring-white/10 backdrop-blur-sm pointer-events-none",
            title: "Build version and commit hash",
            {format!("tensho v{BUILD_VERSION} · {GIT_HASH}")}
        }
    )
}
