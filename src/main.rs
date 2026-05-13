mod card;
mod deck;
mod hand;
mod ocgcore;

use crate::card::Card;
use crate::hand::Hand;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

static OCGCORE_WASM: Asset = asset!(
    "/assets/ocgcore.wasm",
    AssetOptions::builder()
        .with_hash_suffix(false)
        .into_asset_options()
);
static OCGCORE_JS: Asset = asset!(
    "/assets/ocgcore.js",
    AssetOptions::js().with_hash_suffix(false)
);
static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn phase_name(phase: Option<u32>) -> &'static str {
    match phase {
        Some(0x01) => "Draw",
        Some(0x02) => "Standby",
        Some(0x04) => "Main 1",
        Some(0x08) => "Battle Start",
        Some(0x10) => "Battle Step",
        Some(0x20) => "Damage",
        Some(0x40) => "Damage Calc",
        Some(0x80) => "Battle",
        Some(0x100) => "Main 2",
        Some(0x200) => "End",
        Some(_) => "Unknown",
        None => "Waiting",
    }
}

#[component]
fn PhaseIndicator(phase: ReadSignal<Option<u32>>, turn_player: ReadSignal<Option<u8>>) -> Element {
    let phase_text = phase_name(*phase.read());
    let turn_text = match *turn_player.read() {
        Some(turn) => format!("Turn: Player {}", u32::from(turn) + 1),
        None => String::from("Turn: -"),
    };

    rsx! {
        div {
            class: "fixed top-3 right-3 z-50 rounded-lg border border-white/15 bg-[#111827] px-3 py-2 font-mono text-xs text-white shadow-lg shadow-black/25 backdrop-blur",
            p { class: "m-0 font-semibold tracking-wide", "Phase: {phase_text}" }
            p { class: "m-0 text-white/80", "{turn_text}" }
        }
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let _ = (OCGCORE_WASM, OCGCORE_JS);
    let mut hand_contents = use_signal(|| Vec::<String>::new());
    let selected_card = use_signal(|| -1i32);
    let mut current_duel = use_signal(|| None::<u32>);
    let mut current_phase = use_signal(|| None::<u32>);
    let mut current_turn = use_signal(|| None::<u8>);
    let available_summons = use_signal(|| Vec::<u32>::new());
    let mut waiting_for_engine_input = use_signal(|| false);
    let mut waiting_for_zone_selection = use_signal(|| false);
    let mut selected_zone_for_summon = use_signal(|| 0usize);

    // Field zones: Modern Yu-Gi-Oh layout
    // Spell/Trap zones (5 zones)
    let szone_contents = use_signal(|| Vec::<String>::new());
    // Monster zones (5 main + 2 extra)
    let mzone_contents = use_signal(|| Vec::<String>::new());
    let emzone_left = use_signal(|| "0".to_string());
    let emzone_right = use_signal(|| "0".to_string());
    // Other zones
    let field_zone = use_signal(|| "0".to_string());
    let deck_count = use_signal(|| 0usize);
    let gy_contents = use_signal(|| Vec::<String>::new());
    let banish_contents = use_signal(|| Vec::<String>::new());
    let extra_deck_count = use_signal(|| 0usize);

    // Load ocgcore and get version
    let ocgcore_version = use_resource(move || async move {
        ocgcore::load_ocgcore().await.ok()?;
        ocgcore::get_version().ok()
    });

    let normal_summon = move |_| {
        info!("Normal summon button clicked");
        let duel_opt = *current_duel.read();
        let Some(duel) = duel_opt else {
            warn!("No duel active");
            return;
        };

        let hand_index = *selected_card.read();
        let hand_list = hand_contents.read();
        let hand_code = if hand_index >= 0 && (hand_index as usize) < hand_list.len() {
            hand_list[hand_index as usize].parse::<u32>().ok()
        } else {
            hand_list.first().and_then(|code| code.parse::<u32>().ok())
        };

        let Some(hand_code) = hand_code else {
            warn!("Could not parse hand code");
            return;
        };
        info!("Selected card code: 0x{:08x}", hand_code);

        let summons = available_summons.read();
        info!("Available summons count: {}", summons.len());
        if summons.is_empty() {
            warn!("No available summons - waiting for MSG_SELECT_IDLECMD");
            return;
        }

        let summon_index = summons
            .iter()
            .position(|code| *code == hand_code)
            .unwrap_or(0);
        info!(
            "Responding to MSG_SELECT_IDLECMD with summon index {}",
            summon_index
        );
        // Response format for MSG_SELECT_IDLECMD normal summon: (index << 16) + 0
        // Low 16 bits = action type (0 = normal summon)
        // High 16 bits = card index in summon list
        let response = (summon_index as u32) << 16;
        info!("Sending response: 0x{:08x}", response);
        ocgcore::send_response_u32(duel, response);
        waiting_for_engine_input.set(false);
        // Don't call process_duel_step here - let the polling loop handle it
        // This avoids conflicts between the button handler and the polling loop
    };

    let create_duel = move |_| {
        println!("=== Create duel button clicked ===");
        spawn_local(async move {
            match deck::initialize_duel().await {
                Ok(duel_handle) => {
                    println!("Duel initialized with handle: {}", duel_handle);
                    current_duel.set(Some(duel_handle));
                    current_phase.set(None);
                    current_turn.set(None);
                    // Sync the UI from the engine immediately, then keep polling while the duel is active.
                    let cards = ocgcore::query_hand(duel_handle, 0);
                    debug!("Initial hand: {:?}", cards);
                    hand_contents.set(cards);

                let duel_state = current_duel;
                let mut phase_state = current_phase;
                let mut turn_state = current_turn;
                let mut summons_state = available_summons;
                let mut hand_state = hand_contents;
                let mut szone_state = szone_contents;
                let mut mzone_state = mzone_contents;
                let mut emz_left_state = emzone_left;
                let mut emz_right_state = emzone_right;
                let mut fzone_state = field_zone;
                let mut deck_state = deck_count;
                let mut gy_state = gy_contents;
                let mut banish_state = banish_contents;
                let mut extra_deck_state = extra_deck_count;
                let mut engine_wait_state = waiting_for_engine_input;
                let mut waiting_zone_state = waiting_for_zone_selection;
                let mut selected_zone_state = selected_zone_for_summon;
                spawn_local(async move {
                    info!("Polling loop started");

                    // Query field zones - called only when state changes
                    let mut query_field_zones = |duel: u32| {
                        let mzone_cards = ocgcore::query_location_codes(duel, 0, 0x04u32);
                        let mzone_vec: Vec<String> = mzone_cards
                            .iter()
                            .take(5)
                            .map(|code| code.to_string())
                            .collect();
                        mzone_state.set(mzone_vec);

                        emz_left_state.set(mzone_cards.get(5).copied().unwrap_or(0).to_string());
                        emz_right_state.set(mzone_cards.get(6).copied().unwrap_or(0).to_string());

                        let szone_cards = ocgcore::query_location_codes(duel, 0, 0x08u32);
                        let szone_vec: Vec<String> = szone_cards
                            .iter()
                            .take(5)
                            .map(|code| code.to_string())
                            .collect();
                        szone_state.set(szone_vec);

                        fzone_state.set(szone_cards.get(5).copied().unwrap_or(0).to_string());

                        let gy_cards = ocgcore::query_location_codes(duel, 0, 0x10u32);
                        let gy_vec: Vec<String> = gy_cards.into_iter()
                            .filter(|code| *code != 0)
                            .map(|code| code.to_string())
                            .collect();
                        gy_state.set(gy_vec);

                        let banish_cards = ocgcore::query_location_codes(duel, 0, 0x20u32);
                        let banish_vec: Vec<String> = banish_cards.into_iter()
                            .filter(|code| *code != 0)
                            .map(|code| code.to_string())
                            .collect();
                        banish_state.set(banish_vec);

                        deck_state.set(ocgcore::get_deck_count(duel, 0) as usize);
                        extra_deck_state.set(ocgcore::get_extra_deck_count(duel, 0) as usize);
                    };

                    loop {
                        TimeoutFuture::new(100).await;
                        let duel_opt = *duel_state.read();
                        let Some(duel) = duel_opt else {
                            info!("Duel was destroyed");
                            break;
                        };

                        // Only call duelProcess if we're not waiting for input
                        // Return 0 = more to process, 1 = waiting for input, 2 = chain/event needed
                        let step_result = ocgcore::process_duel_step(duel);
                        if step_result != 1 {
                            engine_wait_state.set(false);
                            debug!("duelProcess returned: {}", step_result);
                        }
                        
                        // Log the first waiting tick, but keep advancing the core on later ticks.
                        if step_result == 1 && !*engine_wait_state.read() {
                            debug!("duelProcess returned: 1");
                            engine_wait_state.set(true);
                        }

                        if step_result == 1 && ocgcore::poll_messages(duel).is_none() {
                            if !*engine_wait_state.read() {
                                debug!("duelProcess returned: 1");
                                engine_wait_state.set(true);
                            }
                        }

                        if let Some(messages) = ocgcore::poll_messages(duel) {
                            let message_count = messages.length();
                            for index in 0..message_count {
                                let message = messages.get(index);
                                if let Ok(bytes) = message.dyn_into::<Uint8Array>() {
                                    let data = bytes.to_vec();
                                    if let Some((&msg_id, payload)) = data.split_first() {
                                        // Don't log MSG_RETRY (1) spam - it's just the engine waiting
                                        if msg_id != 1 {
                                            debug!("Message ID: {}", msg_id);
                                        }
                                        match msg_id {
                                            11 => {
                                                // MSG_SELECT_IDLECMD - asking what action to take
                                                // First byte is the player asking (0 = human player, 1 = opponent)
                                                let selecting_player =
                                                    payload.first().copied().unwrap_or(0);

                                                if selecting_player != 0 {
                                                    // Opponent's turn - auto-respond with "end turn" (response 6)
                                                    debug!(
                                                        "MSG_SELECT_IDLECMD for player {} - auto-ending turn",
                                                        selecting_player
                                                    );
                                                    ocgcore::send_response_u32(duel, 6);
                                                    return;
                                                }

                                                debug!(
                                                    "MSG_SELECT_IDLECMD for player 0 (human) - waiting for input"
                                                );
                                                engine_wait_state.set(true);
                                                if payload.len() >= 5 {
                                                    // Normal summon count is u32 at bytes 1-4
                                                    let summon_count = u32::from_le_bytes([
                                                        payload[1], payload[2], payload[3],
                                                        payload[4],
                                                    ])
                                                        as usize;
                                                    debug!("Normal summon count: {}", summon_count);
                                                    let mut summons = Vec::new();
                                                    let mut cursor = 5usize;

                                                    // Parse normal summon cards (each is 10 bytes: code 4, controller 1, location 1, sequence 4)
                                                    for i in 0..summon_count {
                                                        if cursor + 10 > payload.len() {
                                                            warn!(
                                                                "Breaking at summon {}: need 10 bytes, have {}",
                                                                i,
                                                                payload.len() - cursor
                                                            );
                                                            break;
                                                        }
                                                        let code = u32::from_le_bytes([
                                                            payload[cursor],
                                                            payload[cursor + 1],
                                                            payload[cursor + 2],
                                                            payload[cursor + 3],
                                                        ]);
                                                        debug!(
                                                            "Normal summon {}: 0x{:08x}",
                                                            i, code
                                                        );
                                                        summons.push(code);
                                                        cursor += 10;
                                                    }
                                                    debug!(
                                                        "Setting {} available summons",
                                                        summons.len()
                                                    );
                                                    summons_state.set(summons);
                                                } else {
                                                    warn!(
                                                        "MSG_SELECT_IDLECMD payload too short: {}",
                                                        payload.len()
                                                    );
                                                }
                                            }
                                            40 => {
                                                if let Some(turn) = payload.first().copied() {
                                                    println!("MSG_NEW_TURN: player {}", turn);
                                                    turn_state.set(Some(turn));
                                                }
                                            }
                                            41 => {
                                                if payload.len() >= 2 {
                                                    let phase = u16::from_le_bytes([
                                                        payload[0], payload[1],
                                                    ])
                                                        as u32;
                                                    println!(
                                                        "MSG_NEW_PHASE: 0x{:04x} ({})",
                                                        phase,
                                                        phase_name(Some(phase))
                                                    );
                                                    phase_state.set(Some(phase));
                                                }
                                            }
                                            1 => {
                                                // MSG_RETRY - informational, no response needed
                                            }
                                            18 => {
                                                // MSG_SELECT_PLACE - ask which zone to summon into
                                                // Wait for player input instead of auto-responding
                                                debug!("MSG_SELECT_PLACE: waiting for player to select zone");
                                                waiting_zone_state.set(true);
                                                selected_zone_state.set(0); // default to zone 0
                                            }
                                            19 => {
                                                // MSG_SELECT_POSITION - ask what position (face-up attack, etc)
                                                // Already handled in summon response, but respond anyway
                                                debug!(
                                                    "MSG_SELECT_POSITION - sending 0 (face-up attack)"
                                                );
                                                ocgcore::send_response_u32(duel, 0x0);
                                            }
                                            2 => {
                                                // MSG_HINT - informational message, no response needed
                                                debug!("MSG_HINT: payload_len={}", payload.len());
                                                if payload.len() >= 3 {
                                                    let hint_type = payload[0];
                                                    debug!("  hint_type: {}", hint_type);
                                                }
                                            }
                                            5 => {
                                                // MSG_UPDATE_DATA - field state has changed, query zones
                                                debug!("MSG_UPDATE_DATA: querying field zones");
                                                query_field_zones(duel);
                                            }
                                            90 => {
                                                // Unknown/unhandled message type
                                                debug!("MSG_90: payload_len={}", payload.len());
                                            }
                                            16 => {
                                                // MSG_CHAIN - chain info (no response needed)
                                                debug!("MSG_CHAIN: payload_len={}", payload.len());
                                            }
                                            50 => {
                                                // MSG_SUMMON or card summon animation
                                                debug!("MSG_50 (summon/action): payload_len={}", payload.len());
                                            }
                                            60 => {
                                                // MSG_CHAINING or other action
                                                debug!("MSG_60 (chaining): payload_len={}", payload.len());
                                            }
                                            61 => {
                                                // MSG_CHAINING_DISABLED or action end
                                                debug!("MSG_61 (action end): payload_len={}", payload.len());
                                                query_field_zones(duel);
                                            }
                                            _ => {
                                                if msg_id != 1 {
                                                    debug!(
                                                        "Unhandled message: {} (payload_len={})",
                                                        msg_id,
                                                        payload.len()
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Only query hand when we actually have messages to process
                        if step_result != 1 {
                            let cards = ocgcore::query_hand(duel, 0);
                            hand_state.set(cards);
                        }
                    }
                });

                println!("Created duel: {duel_handle}");
            }
                Err(e) => {
                    println!("Failed to create duel: {e}");
                }
            }
        });
    };

    let destroy_duel = move |_| {
        let duel_opt = *current_duel.read();
        if let Some(duel) = duel_opt {
            ocgcore::destroyDuel(duel);
            current_duel.set(None);
            current_phase.set(None);
            current_turn.set(None);
            hand_contents.set(Vec::new());
            println!("Destroyed duel");
        }
    };

    let duel_opt = *current_duel.read();

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        main {
            class: "h-dvh w-dvw",

            // Display OCG version
            div {
                match &*ocgcore_version.read() {
                    Some(Some((major, minor))) => rsx! {
                        p { "OCG Version: {major}.{minor}" }
                    },
                    Some(None) => rsx! { p { "Version error" } },
                    None => rsx! { p { "Loading version..." } },
                }
            }

            // Display duel status
            div {
                match duel_opt {
                    Some(duel) => rsx! {
                        p { "Duel active (handle: {duel})" }
                        button {
                            class: "w-[10vw] aspect-[3/1] bg-[lightcoral]",
                            onclick: destroy_duel,
                            "Destroy Duel"
                        }
                    },
                    None => rsx! {
                        p { "No duel" }
                        button {
                            class: "w-[10vw] aspect-[3/1] bg-[lightgreen]",
                            onclick: create_duel,
                            "Create Duel"
                        }
                    },
                }
            }

            PhaseIndicator {
                phase: current_phase,
                turn_player: current_turn,
            }

            // Modern Yu-Gi-Oh Field Layout (EDOPro/Master Duel style)
            div {
                class: "mt-6 mx-4 flex flex-row gap-6 bg-gradient-to-b from-slate-950 to-slate-900 p-6 rounded-lg border border-slate-700",

                // LEFT SIDE: Extra Deck and Field Zone
                div {
                    class: "flex flex-col gap-4 items-center",
                    // Extra Deck
                    div {
                        class: "flex flex-col items-center gap-2",
                        div {
                            class: "relative w-20 aspect-[59/86] border-2 border-slate-500 rounded bg-slate-800 flex items-center justify-center cursor-pointer hover:border-blue-400",
                            p { class: "text-xs text-gray-400 font-bold", "{extra_deck_count}" }
                        }
                        p { class: "text-xs text-gray-500 text-center", "Extra\nDeck" }
                    }
                    // Field Zone
                    div {
                        class: "flex flex-col items-center gap-2",
                        div {
                            class: "relative w-20 aspect-[59/86] border-2 border-purple-600 rounded bg-slate-800 flex items-center justify-center",
                            if *field_zone.read() == "0" {
                                p { class: "text-xs text-gray-500", "Field" }
                            } else {
                                Card { id: field_zone.read().clone() }
                            }
                        }
                        p { class: "text-xs text-gray-500 text-center", "Field\nZone" }
                    }
                }

                // CENTER: Monster and Spell/Trap Zones
                div {
                    class: "flex flex-col gap-2 flex-1",
                    // EMZ row (positioned above MMZ 1 and MMZ 3)
                    div {
                        class: "flex flex-row justify-center gap-2",
                        // Empty space above MMZ 0
                        div { class: "w-24" }
                        // EMZ Left (above MMZ 1)
                        div {
                            class: "relative w-24 aspect-[59/86] border-2 border-yellow-500 rounded-lg bg-slate-700 flex items-center justify-center",
                            if *emzone_left.read() == "0" {
                                p { class: "text-xs text-gray-500 text-center font-bold", "EMZ" }
                            } else {
                                Card { id: emzone_left.read().clone() }
                            }
                        }
                        // Empty space above MMZ 2
                        div { class: "w-24" }
                        // EMZ Right (above MMZ 3)
                        div {
                            class: "relative w-24 aspect-[59/86] border-2 border-yellow-500 rounded-lg bg-slate-700 flex items-center justify-center",
                            if *emzone_right.read() == "0" {
                                p { class: "text-xs text-gray-500 text-center font-bold", "EMZ" }
                            } else {
                                Card { id: emzone_right.read().clone() }
                            }
                        }
                        // Empty space above MMZ 4
                        div { class: "w-24" }
                    }

                    // Monster Zones (5 Main Monster Zones)
                    div {
                        class: "flex flex-row justify-center gap-2",
                        for (idx, card_id) in mzone_contents.read().iter().enumerate() {
                            {
                                let is_selected = waiting_for_zone_selection() && selected_zone_for_summon() == idx;
                                let border_class = if waiting_for_zone_selection() {
                                    if is_selected {
                                        "border-4 border-green-500"
                                    } else {
                                        "border-2 border-blue-500 hover:border-green-400"
                                    }
                                } else {
                                    "border-2 border-blue-500"
                                };
                                let bg_class = if is_selected {
                                    "bg-green-900/50"
                                } else if waiting_for_zone_selection() {
                                    "bg-slate-700 hover:bg-slate-600"
                                } else {
                                    "bg-slate-800 hover:bg-slate-700"
                                };

                                rsx! {
                                    div {
                                        class: "relative w-24 aspect-[59/86] rounded flex items-center justify-center cursor-pointer transition-colors",
                                        class: "{border_class}",
                                        class: "{bg_class}",
                                        onclick: move |_| {
                                            if waiting_for_zone_selection.read().clone() {
                                                info!("Zone selected for summon: {}", idx);
                                                let duel_opt = *current_duel.read();
                                                if let Some(duel) = duel_opt {
                                                    selected_zone_for_summon.set(idx);
                                                    // Response format for MSG_SELECT_PLACE: 3 bytes
                                                    // byte 0: player (0=current, 1=opponent)
                                                    // byte 1: location (0x04=MZONE)
                                                    // byte 2: sequence (zone index)
                                                    let response = [0u8, 0x04u8, idx as u8];
                                                    info!("Sending MSG_SELECT_PLACE response: zone {}", idx);
                                                    ocgcore::duelSetResponse(duel, &response);
                                                    waiting_for_engine_input.set(false);
                                                    waiting_for_zone_selection.set(false);
                                                }
                                            }
                                        },
                                        if *card_id == "0" {
                                            p {
                                                class: if waiting_for_zone_selection() { "text-xs text-green-300 font-bold" } else { "text-xs text-gray-500 font-bold" },
                                                "M{idx}"
                                            }
                                        } else {
                                            Card { id: card_id.clone() }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Spell & Trap Zones (below MMZ)
                    div {
                        class: "flex flex-row justify-center gap-2",
                        for (idx, card_id) in szone_contents.read().iter().enumerate() {
                            div {
                                class: "relative w-24 aspect-[59/86] border-2 border-slate-500 rounded bg-slate-800 flex items-center justify-center hover:bg-slate-700 cursor-pointer",
                                if *card_id == "0" {
                                    p { class: "text-xs text-gray-500 text-sm", "S{idx}" }
                                } else {
                                    Card { id: card_id.clone() }
                                }
                            }
                        }
                    }
                }

                // RIGHT SIDE: Banishment, Graveyard, and Deck
                div {
                    class: "flex flex-col gap-4 items-center",
                    // Banishment
                    div {
                        class: "flex flex-col items-center gap-2",
                        div {
                            class: "relative w-20 aspect-[59/86] border-2 border-slate-500 rounded bg-slate-800 flex items-center justify-center cursor-pointer hover:border-blue-400",
                            if banish_contents.read().is_empty() {
                                p { class: "text-xs text-gray-500", "Ban" }
                            } else {
                                Card { id: banish_contents.read().last().unwrap().clone() }
                            }
                        }
                        p { class: "text-xs text-gray-500 text-center", "Banish\n({banish_contents.read().len()})" }
                    }
                    // Graveyard
                    div {
                        class: "flex flex-col items-center gap-2",
                        div {
                            class: "relative w-20 aspect-[59/86] border-2 border-slate-500 rounded bg-slate-800 flex items-center justify-center cursor-pointer hover:border-blue-400",
                            if gy_contents.read().is_empty() {
                                p { class: "text-xs text-gray-500", "GY" }
                            } else {
                                Card { id: gy_contents.read().last().unwrap().clone() }
                            }
                        }
                        p { class: "text-xs text-gray-500 text-center", "GY\n({gy_contents.read().len()})" }
                    }
                    // Deck
                    div {
                        class: "flex flex-col items-center gap-2",
                        div {
                            class: "relative w-20 aspect-[59/86] border-2 border-slate-500 rounded bg-slate-800 flex items-center justify-center cursor-pointer hover:border-blue-400",
                            p { class: "text-xs text-gray-400 font-bold", "{deck_count}" }
                        }
                        p { class: "text-xs text-gray-500 text-center", "Deck" }
                    }
                }
            }

            Hand {
                cards: hand_contents,
                selected_card: selected_card,
                available_summons: available_summons,
                on_normal_summon: normal_summon,
            }
        }
    }
}
