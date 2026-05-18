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
        main {
            class: "h-dvh w-dvw bg-slate-800",
            Field {
                monsters: monsters
            }
            Hand {
                cards: hand_contents,
                selected_card: selected_card,
                normal_summons: normal_summons,
                hand_actions
            }
        }
    )
}
