use dioxus::prelude::*;

use crate::ocgcore::OCGCore;
use crate::utility::cache_scripts;
use crate::utility::get_cached_script;

static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    let mut duel_state = use_signal(|| String::from("Initializing..."));
    let mut hand_state = use_signal(|| Vec::new());

    let _ = use_resource(move || async move {
        let core = consume_context::<OCGCore>();

        let duel = core.create_duel().unwrap();

        cache_scripts(Vec::from([90681088])).await;

        let constants_script = get_cached_script("constant.lua").unwrap();
        duel.load_script(constants_script, "constant.lua").unwrap();

        let utility_script = get_cached_script("utility.lua").unwrap();
        duel.load_script(utility_script, "utility.lua").unwrap();

        duel.add_card(0, 0, 90681088, 0, 0x01, 0, 0).unwrap();
        duel.start().unwrap();

        loop {
            let status = duel.process();

            if status == 1 {
                if let Ok(Some(msg)) = duel.get_message() {
                    let msg_type = msg.get_index(2);

                    if msg_type == 11 {
                        duel_state.set("IDLE_CMD".to_string());
                        break;
                    }
                }
            }

            if status == 2 {
                continue;
            }

            break;
        }

        hand_state.set(duel.query_hand(0));
    });

    rsx!(
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div { class: "p-8",
            div { class: "mt-4 font-mono text-xs", "{duel_state:#?} - {hand_state:#?}" }
        }
    )
}
