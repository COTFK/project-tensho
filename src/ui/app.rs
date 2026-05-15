use dioxus::prelude::*;

use crate::ocgcore::OCGCore;

static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    // 1. Initialize the duel state
    let mut duel_state = use_signal(|| String::from("Initializing..."));
    let mut hand_state = use_signal(|| Vec::new());

    // 2. Run the duel logic in a resource so it doesn't block the UI
    let _ = use_resource(move || async move {
        let core = use_context::<OCGCore>();
        let duel = core.create_duel().unwrap();
        // Add cards (e.g., to Deck)
        // 0x01 = DECK, 0x02 = HAND
        duel.add_card(0, 0, 90681088, 0, 0x01, 0, 0).ok();

        duel.start().expect("Failed to start");

        loop {
            let status = duel.process();
            tracing::debug!("status {status}");

            // If status is 1 (WAITING), check the message
            if status == 1 {
                if let Ok(Some(msg)) = duel.get_message() {
                    let raw_hex = msg.to_vec().iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<String>>()
                        .join(" ");

                    // Look for the "50" (MSG_MOVE) or "11" (MSG_IDLE) and count the bytes manually
                    tracing::info!("RAW BUFFER: {}", raw_hex);


                    let msg_type = msg.get_index(0);
                    // tracing::debug!("{:#?}", msg.to_js_string());

                    // MSG_SELECT_IDLECMD (0x0e) usually means Main Phase is active
                    if msg_type == 0x0e {
                        duel_state.set("Main Phase 1 reached!".to_string());
                        break;
                    }

                    // MSG_SELECT_TP (0x01) - Who goes first?
                    if msg_type == 0x01 {
                        // Response: 0 for Team 1, 1 for Team 2
                        duel.set_response(&[0]).ok();
                        continue; // Process again after responding
                    }
                }
            }

            // If status is 2, the core is still working; just continue
            if status == 2 {
                continue;
            }

            // If it's something else, stop to prevent infinite loops
            break;
        }

        hand_state.set(duel.query_hand(0));
    });

    rsx!(
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div { class: "p-8",
            h1 { class: "text-2xl font-bold", "Duel Monitor" }
            p { "Status: {duel_state}" }
            p { "Hand Size: {hand_state.read().len()}" }
            div { class: "mt-4 font-mono text-xs", "{hand_state:#?}" }
        }
    )
}
