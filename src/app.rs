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
use crate::utility::CardLabel;
use crate::utility::get_cached_label;

#[component]
pub fn App(duel: Duel) -> Element {
    use_context_provider(move || duel);

    let mut hand_contents = use_signal(Vec::new);
    let mut monsters = use_signal(Vec::new);
    let mut selected_card = use_signal(|| -1);
    let mut card_effect_prompt = use_signal(|| 0u32);
    let mut hand_chainables = use_signal(HashMap::new);

    // global input toggle or something
    let mut waiting_on_input = use_signal(|| false);
    use_context_provider(move || waiting_on_input);

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
            HandAction::Chain {
                card_code,
                sequence,
            } => {
                debug!("Chain requested for card {card_code} at index {sequence}");

                let response = [sequence, 0, 0, 0];
                duel.set_response(&response);
                waiting_on_input.set(false);
            }
        }

        selected_card.set(-1);
        normal_summons.clear();
        hand_chainables.clear();
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
                                if messages.len() < 20 {
                                    return;
                                }

                                let player = messages[5];
                                let count: usize;

                                if messages.len() == 20 {
                                    count = 0;
                                } else {
                                    count = u32::from_le_bytes([
                                        messages[16],
                                        messages[17],
                                        messages[18],
                                        messages[19],
                                    ]) as usize;
                                }

                                debug!("SelectChain -> Player: {}, Count: {}", player, count);

                                if count == 0 {
                                    debug!(
                                        "No selectable chain choices present. Declining/Passing priority."
                                    );
                                    let response: [u8; 4] = [255, 255, 255, 255];
                                    duel.set_response(&response);
                                    waiting_on_input.set(false);
                                } else {
                                    debug!(
                                        "Found {} chain options! Parsing target options array...",
                                        count
                                    );

                                    let mut offset = 20;

                                    for chain_option in 0..count {
                                        if offset + 7 <= messages.len() {
                                            let card_code = u32::from_le_bytes([
                                                messages[offset],
                                                messages[offset + 1],
                                                messages[offset + 2],
                                                messages[offset + 3],
                                            ]);

                                            let controller = messages[offset + 4];
                                            let location_bit = messages[offset + 5];
                                            let sequence = messages[offset + 6];

                                            // Safe decoding pattern to prevent unwrap crashes on unexpected bytes
                                            let location = CardLocation::try_from(location_bit).unwrap();

                                            debug!(
                                                "  Option #{}: Card ID {}, Controller: {}, Location: {:?}, Slot: {}",
                                                chain_option, card_code, controller, location, sequence
                                            );

                                            match location {
                                                CardLocation::Hand => {
                                                    hand_chainables.with_mut(|v| v.insert(sequence, chain_option));
                                                }
                                                _ => {}
                                            }
                                        }

                                        // Advance by 23 bytes to cleanly clear the trailing descriptive payload fields
                                        offset += 23;
                                    }

                                    waiting_on_input.set(true);
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

                                card_effect_prompt.set(card_code);
                                eval("eff_dialog.show();");
                                waiting_on_input.set(true);
                            }
                            CoreMessage::SelectCard => {}
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
        main {
            class: "h-dvh w-dvw bg-gray-800",
            Field { monsters }
            Hand {
                cards: hand_contents,
                selected_card,
                normal_summons,
                hand_chainables: hand_chainables,
                hand_actions,
            }
            ActivateEffectModal {card_id: card_effect_prompt}
        }
    )
}

#[component]
pub fn ActivateEffectModal(card_id: ReadSignal<u32>) -> Element {
    let card_labels = get_cached_label(card_id()).unwrap_or(CardLabel {
        name: String::from("unknown"),
    });

    let activate_effect = move |_| {
        let duel = use_context::<Duel>();
        let mut waiting_on_input = use_context::<Signal<bool>>();

        let response: [u8; 4] = [1, 0, 0, 0];
        duel.set_response(&response);
        waiting_on_input.set(false);
    };
    let refuse_to_activate = move |_| {
        let duel = use_context::<Duel>();
        let mut waiting_on_input = use_context::<Signal<bool>>();

        let response: [u8; 4] = [0, 0, 0, 0];
        duel.set_response(&response);
        waiting_on_input.set(false);
    };

    rsx!(
        el-dialog {
            dialog {
                aria_labelledby: "dialog-title",
                class: "fixed inset-0 size-auto max-h-none max-w-none overflow-y-auto bg-transparent backdrop:bg-transparent",
                id: "eff_dialog",
                div {
                    class: "flex min-h-full items-end justify-center p-4 text-center focus:outline-none sm:items-center sm:p-0",
                    tabindex: "0",
                    el-dialog-panel { class: "relative transform overflow-hidden rounded-lg bg-gray-800 text-left shadow-xl outline -outline-offset-1 outline-white/10 transition-all data-closed:translate-y-4 data-closed:opacity-0 data-enter:duration-300 data-enter:ease-out data-leave:duration-200 data-leave:ease-in sm:my-8 sm:w-full sm:max-w-lg data-closed:sm:translate-y-0 data-closed:sm:scale-95",
                        div { class: "bg-gray-800 px-4 pt-5 pb-4 sm:p-6 sm:pb-4",
                            div { class: "sm:flex sm:items-start",
                                div { class: "mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left",
                                    h3 {
                                        class: "text-base font-semibold text-white",
                                        id: "dialog-title",
                                        "Activate effect?"
                                    }
                                    div { class: "mt-2",
                                        p { class: "text-sm text-gray-400",
                                            "Activate the effect of {card_labels.name}?"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "bg-gray-700/25 px-4 py-3 flex flex-row w-full gap-2",
                            button {
                                class: "grow-1 inline-flex w-full justify-center rounded-md bg-green-500 px-3 py-2 text-sm font-semibold text-white hover:bg-green-800",
                                "command": "close",
                                "commandfor": "eff_dialog",
                                r#type: "button",
                                onclick: activate_effect,
                                "Yes"
                            }
                            button {
                                class: "grow-1 inline-flex w-full justify-center rounded-md bg-red-500 px-3 py-2 text-sm font-semibold text-white hover:bg-red-800",
                                "command": "close",
                                "commandfor": "eff_dialog",
                                r#type: "button",
                                onclick: refuse_to_activate,
                                "No"
                            }
                        }
                    }
                }
            }
        }
    )
}
