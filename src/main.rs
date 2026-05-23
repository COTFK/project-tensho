mod ocgcore;
mod state;
mod ui;
mod utility;

use dioxus::prelude::*;
use rand::seq::SliceRandom;

use crate::ocgcore::OCGCore;
use crate::ocgcore::constants::*;
use crate::ui::DuelScreen;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_labels;
use crate::utility::cache_scripts;
use crate::utility::get_cached_script;

fn main() {
    dioxus::launch(AppContainer);
}

#[component]
pub fn AppContainer() -> Element {
    let core_resource = use_resource(async move || {
        // Set up deck
        let mut main_deck = MAIN_DECK_IDS;
        main_deck.shuffle(&mut rand::rng());

        let mut all_cards = Vec::from(MAIN_DECK_IDS);
        all_cards.append(&mut Vec::from(EXTRA_DECK_IDS));
        cache_scripts(&all_cards).await;
        cache_labels(&all_cards).await;

        let core = OCGCore::load().await?;
        let duel = core.create_duel().unwrap();

        duel.load_script(get_cached_script("constant.lua").unwrap(), "constant.lua");
        duel.load_script(get_cached_script("utility.lua").unwrap(), "utility.lua");

        for card_id in main_deck {
            duel.add_card(
                CardOwner::Player,
                card_id,
                CardController::Player,
                CardLocation::Deck,
                0,
                0,
            );
        }

        duel.start();
        debug!("Duel started successfully.");

        anyhow::Ok(duel)
    });

    rsx!(
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        match &*core_resource.read() {
            Some(Ok(duel)) => rsx!(DuelScreen { duel: duel.clone() }),
            Some(Err(e)) => rsx!("{e:#?}"),
            None => {
                rsx!("Loading...")
            }
        }
    )
}
