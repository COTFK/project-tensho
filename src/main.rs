mod app;
mod ocgcore;
mod ui;
mod utility;

use dioxus::prelude::*;

use crate::app::App;
use crate::ocgcore::OCGCore;

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
