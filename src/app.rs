use dioxus::prelude::*;
use js_sys::Uint8Array;
use rand::seq::SliceRandom;
use wasm_bindgen::JsCast;

use crate::ocgcore::OCGCore;
use crate::ocgcore::constants::*;
use crate::ui::Hand;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_scripts;
use crate::utility::get_cached_script;

static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    let mut duel_state = use_signal(|| String::from("Initializing..."));
    let mut hand_contents = use_signal(|| Vec::new());
    let selected_card = use_signal(|| -1);

    let _ = use_resource(move || async move {
        let core = consume_context::<OCGCore>();

        let duel = core.create_duel().unwrap();

        let mut cards = Vec::from(MAIN_DECK_IDS);
        cards.append(&mut Vec::from(EXTRA_DECK_IDS));

        cache_scripts(cards).await;

        let constants_script = get_cached_script("constant.lua").unwrap();
        let utility_script = get_cached_script("utility.lua").unwrap();
        duel.load_script(constants_script, "constant.lua").unwrap();
        duel.load_script(utility_script, "utility.lua").unwrap();

        let mut shuffled = MAIN_DECK_IDS.clone();
        shuffled.shuffle(&mut rand::rng());

        for card_id in shuffled {
            duel.add_card(0, 0, card_id, 0, LOCATION_DECK, 0, 0)
                .unwrap();
        }

        duel.start().unwrap();

        loop {
            let status = duel.process();
            if status == 1 {
                if let Some(messages) = duel.poll_messages() {
                    for message in messages.iter() {
                        let msg: Uint8Array = message.unchecked_into();
                        if msg.length() > 0 && msg.get_index(0) == 11 {
                            duel_state.set("IDLE_CMD".to_string());
                            break;
                        }
                    }
                }

                if duel_state.read().as_str() == "IDLE_CMD" {
                    break;
                }

                continue;
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
            class: "h-dvh w-dvw",
            Hand {
                cards: hand_contents,
                selected_card: selected_card,
            }
        }
    )
}
