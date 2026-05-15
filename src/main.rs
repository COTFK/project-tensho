mod ocgcore;
mod ui;

use dioxus::prelude::*;

use crate::ocgcore::OCGCore;

static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

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
                let (major, minor) = core.get_version().unwrap();

                rsx!("core v{major}.{minor}")
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
