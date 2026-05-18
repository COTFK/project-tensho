use std::collections::HashMap;

use dioxus::document::eval;
use dioxus::prelude::*;

use crate::ocgcore::Duel;
use crate::ocgcore::DuelStatus;
use crate::ocgcore::HandAction;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::CoreMessage;
use crate::ui::Field;
use crate::ui::Hand;

#[component]
pub fn App(duel: Duel) -> Element {
    use_context_provider(move || duel);

    let mut hand_contents = use_signal(Vec::new);
    let mut monsters = use_signal(Vec::new);
    let mut selected_card = use_signal(|| -1);
    let mut waiting_on_input = use_signal(|| false);
    let mut card_effect_prompt = use_signal(|| 0u32);

    // Actions
    let mut normal_summons = use_signal(|| HashMap::new());

    let hand_actions = move |action: HandAction| {
        let duel = use_context::<Duel>();

        match action {
            HandAction::NormalSummon {
                card_code,
                summon_index,
            } => {
                debug!("Normal Summon requested for card {card_code} at index {summon_index}");

                let response = ((u32::from(summon_index)) << 16).to_le_bytes();
                duel.set_response(&response);
                waiting_on_input.set(false);
            }
        }

        selected_card.set(-1);
        normal_summons.clear();
    };

    use_effect(move || {
        let duel = use_context::<Duel>();

        if !waiting_on_input() {
            loop {
                match duel.process() {
                    DuelStatus::Awaiting => {
                        let messages = duel.get_messages();
                        let msg_byte = messages.get(4).unwrap();
                        let msg_type = CoreMessage::try_from(*msg_byte).unwrap();

                        debug!("Current message is {msg_type:?} with bytes {messages:?}");

                        match msg_type {
                            CoreMessage::Retry => {
                                debug!("Received Retry - this shouldn't happen.");
                            }
                            CoreMessage::Idle => {
                                let actions = duel.get_available_actions(messages).unwrap();

                                normal_summons.set(actions.get_normal_summons());
                            }
                            CoreMessage::SelectPlace => {
                                debug!("SelectPlace received — sending placement response");
                                let response = [
                                    CardController::Player as u8,
                                    CardLocation::MonsterZone as u8,
                                    0,
                                ];
                                duel.set_response(&response);
                                waiting_on_input.set(false);
                            }
                            CoreMessage::SelectChain => {
                                if messages.len() < 12 {
                                    debug!(
                                        "SelectChain packet too short: {} bytes",
                                        messages.len()
                                    );
                                    return;
                                }

                                let player = messages[5];

                                // Read count as a single byte (u8) instead of a u32
                                let count = messages[6] as usize;

                                let special_count = messages[7];

                                // The remaining bytes contain forced flags and hint timings depending on length
                                debug!(
                                    "SelectChain -> Player: {}, Count: {}, Special Count: {}",
                                    player, count, special_count
                                );

                                if count == 0 {
                                    debug!(
                                        "No selectable chain choices present. Declining/Passing priority."
                                    );
                                    let response: [u8; 4] = [255, 255, 255, 255]; // -1 to pass
                                    duel.set_response(&response);
                                    waiting_on_input.set(false);
                                } else {
                                    debug!(
                                        "Found {} chain options! Parsing target options array...",
                                        count
                                    );

                                    // If count > 0, each option block is 12 bytes long, starting right after the header
                                    // Header is: MsgType(1) + Player(1) + Count(1) + SpeCount(1) + Forced(1) + Timings...
                                    // Let's protect the loop with a safe dynamic layout boundary check
                                    let mut offset = 12;

                                    for i in 0..count {
                                        if offset + 4 <= messages.len() {
                                            let card_code = u32::from_le_bytes([
                                                messages[offset],
                                                messages[offset + 1],
                                                messages[offset + 2],
                                                messages[offset + 3],
                                            ]);
                                            debug!("  Option #{}: Card ID {}", i, card_code);
                                        }
                                        offset += 12; // Advance to next choice item block
                                    }

                                    waiting_on_input.set(true); // Stop automated responses and prompt user UI
                                }
                            }
                            CoreMessage::SelectEffectYN => {
                                if messages.len() < 18 {
                                    debug!(
                                        "SelectEffectYN packet too small: {} bytes",
                                        messages.len()
                                    );
                                    return;
                                }

                                let player = messages[5];

                                // Parse the target card triggering its choice prompt
                                let card_code = u32::from_le_bytes([
                                    messages[6],
                                    messages[7],
                                    messages[8],
                                    messages[9],
                                ]);
                                let location = messages[11];
                                let sequence = messages[12];

                                debug!(
                                    "SelectEffectYN -> Player: {}, Card: {}, Location Bit: {}, Zone Index: {}",
                                    player, card_code, location, sequence
                                );

                                // // Automation Option: For testing, let's say "Yes" automatically
                                // // to force the core to process and execute the effect.
                                // debug!("Automatically accepting effect trigger [YES]");
                                // let response: [u8; 4] = [1, 0, 0, 0];
                                // duel.set_response(&response);

                                // If you want your UI to click Yes/No manually later, use this instead:
                                card_effect_prompt.set(card_code);
                                eval("eff_dialog.show();");
                                waiting_on_input.set(true);
                            }
                        }

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

            monsters.set(duel.get_cards(CardLocation::MonsterZone));
            hand_contents.set(duel.get_cards(CardLocation::Hand));
        }
    });

    rsx!(
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        main { class: "h-dvh w-dvw bg-slate-800",
            Field { monsters }
            Hand {
                cards: hand_contents,
                selected_card,
                normal_summons,
                hand_actions,
            }
        }
        el-dialog {
            dialog {
                aria_labelledby: "dialog-title",
                class: "fixed inset-0 size-auto max-h-none max-w-none overflow-y-auto bg-transparent backdrop:bg-transparent",
                id: "eff_dialog",
                // el-dialog-backdrop { class: "fixed inset-0 bg-gray-900/50 transition-opacity data-closed:opacity-0 data-enter:duration-300 data-enter:ease-out data-leave:duration-200 data-leave:ease-in" }
                div {
                    class: "flex min-h-full items-end justify-center p-4 text-center focus:outline-none sm:items-center sm:p-0",
                    tabindex: "0",
                    el-dialog-panel { class: "relative transform overflow-hidden rounded-lg bg-gray-800 text-left shadow-xl outline -outline-offset-1 outline-white/10 transition-all data-closed:translate-y-4 data-closed:opacity-0 data-enter:duration-300 data-enter:ease-out data-leave:duration-200 data-leave:ease-in sm:my-8 sm:w-full sm:max-w-lg data-closed:sm:translate-y-0 data-closed:sm:scale-95",
                        div { class: "bg-gray-800 px-4 pt-5 pb-4 sm:p-6 sm:pb-4",
                            div { class: "sm:flex sm:items-start",
                                div { class: "mx-auto flex size-12 shrink-0 items-center justify-center rounded-full bg-red-500/10 sm:mx-0 sm:size-10",
                                    svg {
                                        // aria_hidden: "true",
                                        class: "size-6 text-red-400",
                                        "data-slot": "icon",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.5",
                                        view_box: "0 0 24 24",
                                        path {
                                            d: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z",
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                        }
                                    }
                                }
                                div { class: "mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left",
                                    h3 {
                                        class: "text-base font-semibold text-white",
                                        id: "dialog-title",
                                        "Deactivate account"
                                    }
                                    div { class: "mt-2",
                                        p { class: "text-sm text-gray-400",
                                            "Are you sure you want to deactivate your account? All of your data will be permanently removed. This action cannot be undone."
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "bg-gray-700/25 px-4 py-3 sm:flex sm:flex-row-reverse sm:px-6",
                            button {
                                class: "inline-flex w-full justify-center rounded-md bg-red-500 px-3 py-2 text-sm font-semibold text-white hover:bg-red-400 sm:ml-3 sm:w-auto",
                                "command": "close",
                                "commandfor": "eff_dialog",
                                r#type: "button",
                                "Deactivate"
                            }
                            button {
                                class: "mt-3 inline-flex w-full justify-center rounded-md bg-white/10 px-3 py-2 text-sm font-semibold text-white inset-ring inset-ring-white/5 hover:bg-white/20 sm:mt-0 sm:w-auto",
                                "command": "close",
                                "commandfor": "eff_dialog",
                                r#type: "button",
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
        document::Script {
            src: "https://cdn.jsdelivr.net/npm/@tailwindplus/elements@1",
            r#type: "module",
        }
    )
}
