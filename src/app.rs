use dioxus::prelude::*;
use js_sys::Uint8Array;
use rand::seq::SliceRandom;
use wasm_bindgen::JsCast;

use crate::ocgcore::OCGCore;
use crate::ocgcore::constants::*;
use crate::ocgcore::IdleCommandPayload;
use crate::ui::Field;
use crate::ui::Hand;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_scripts;
use crate::utility::get_cached_script;

static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    let mut hand_contents = use_signal(|| Vec::new());
    let selected_card = use_signal(|| -1);

    let _ = use_resource(move || async move {
        let core = consume_context::<OCGCore>();

        let duel = core.create_duel().unwrap();

        let mut main_deck = MAIN_DECK_IDS.clone();
        main_deck.shuffle(&mut rand::rng());

        let mut all_cards = Vec::from(MAIN_DECK_IDS);
        all_cards.append(&mut Vec::from(EXTRA_DECK_IDS));

        cache_scripts(all_cards).await;

        let constants_script = get_cached_script("constant.lua").unwrap();
        let utility_script = get_cached_script("utility.lua").unwrap();
        duel.load_script(constants_script, "constant.lua").unwrap();
        duel.load_script(utility_script, "utility.lua").unwrap();

        for card_id in main_deck {
            duel.add_card(0, 0, card_id, 0, LOCATION_DECK, 0, 0)
                .unwrap();
        }

        duel.start().unwrap();

        loop {
            let status = duel.process();
            if status == 1 {
                let messages = duel.get_message().unwrap().to_vec();

                if messages[4] != 11 {
                    warn!("The messages do not contain IDLECMD!");
                }


                let payload = IdleCommandPayload::try_from(&messages[..]);
                debug!("{payload:?}");
                
                break;
            }

            if status == 2 {
                continue;
            }

            break;
        }

        hand_contents.set(duel.query_hand(0));
    });

    rsx!(
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        main {
            class: "h-dvh w-dvw bg-slate-800",
            // Field {}
            Hand {
                cards: hand_contents,
                selected_card: selected_card,
            }
        }
    )
}
