use std::collections::HashSet;

use dioxus::prelude::*;
use rand::seq::SliceRandom;

use crate::ocgcore::DuelStatus;
use crate::ocgcore::OCGCore;
use crate::ocgcore::constants::{CardController, CardLocation, CardOwner};
use crate::ui::Hand;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_scripts;
use crate::utility::get_cached_script;

static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App(core: OCGCore) -> Element {
    use_context_provider(|| core);
    let mut hand_contents = use_signal(Vec::new);
    let selected_card = use_signal(|| -1);

    // Actions
    let mut normal_summons = use_signal(|| HashSet::new());

    use_resource(move || async move {
        let core = consume_context::<OCGCore>();

        let duel = core.create_duel().unwrap();

        let mut main_deck = MAIN_DECK_IDS;
        main_deck.shuffle(&mut rand::rng());

        let mut all_cards = Vec::from(MAIN_DECK_IDS);
        all_cards.append(&mut Vec::from(EXTRA_DECK_IDS));

        cache_scripts(all_cards).await;
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

        loop {
            match duel.process() {
                DuelStatus::Awaiting => {
                    let actions = duel.get_available_actions().unwrap();
                    normal_summons.set(actions.get_normal_summons());
                    break;
                }
                DuelStatus::Continue => {
                    continue;
                }
                DuelStatus::End => {
                    break;
                }
            }
        }

        hand_contents.set(duel.get_cards(CardLocation::Hand));
    });

    rsx!(
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        main {
            class: "h-dvh w-dvw bg-slate-800",
            Hand {
                cards: hand_contents,
                selected_card: selected_card,
                normal_summons: normal_summons
            }
        }
    )
}
