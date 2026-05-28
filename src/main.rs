mod ocgcore;
mod state;
mod ui;
mod utility;

use dioxus::prelude::*;

use crate::state::cache_dependencies;
use crate::state::load_duel;
use crate::ui::DuelScreen;
use crate::ui::LoadingScreen;
use crate::ui::RotateDeviceOverlay;
use crate::utility::BUILD_VERSION;
use crate::utility::GIT_HASH;

fn main() {
    dioxus::launch(AppContainer);
}

#[component]
pub fn AppContainer() -> Element {
    let cache_resource = use_resource(cache_dependencies);

    let core_resource = use_resource(move || load_duel(cache_resource));

    rsx!(
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
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
            class: "fixed bottom-3 left-3 z-50 rounded-md bg-gray-950/70 px-2 py-1 font-mono text-gray-200 shadow-lg ring-1 ring-white/10 backdrop-blur-sm pointer-events-none",
            style: "font-size: 12px",
            title: "Build version and commit hash",
            {format!("burning_draw v{BUILD_VERSION} · {GIT_HASH}")}
        }
    )
}
