mod ocgcore;
mod state;
mod ui;
mod utility;

use dioxus::prelude::*;

use crate::ui::DuelScreen;
use crate::ui::LoadingScreen;
use crate::state::load_duel;
use crate::utility::cache_card_data;

fn main() {
    dioxus::launch(AppContainer);
}

#[component]
pub fn AppContainer() -> Element {
    let cache_resource = use_resource(cache_card_data);

    let core_resource = use_resource(move || load_duel(cache_resource));

    rsx!(
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
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
    )
}
