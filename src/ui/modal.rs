use anyhow::anyhow;
use dioxus::prelude::*;

use super::components::ActivateButton;
use super::components::BlockButton;
use super::components::CardPickerModal;
use super::components::MessageModal;
use super::components::SelectableCard;
use crate::ocgcore::ActiveCard;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_user_response;

#[component]
pub fn ModalContainer() -> Element {
    let state = use_context::<DuelState>();

    rsx!(
        MessageModal {
            trigger: state.card_prompting_to_activate.iter().any(|card| card.chain_option.is_some()),
            title: "A card or effect can be activated. Activate?",
            BlockButton {
                label: "No",
                onclick: |_| send_user_response(UserResponse::PassPriority),
                additional_classes: "bg-red-600/70",
            }
        }
        MessageModal {
            trigger: !state.card_prompting_to_activate.iter().any(|card| card.chain_option.is_some()) && !state.card_prompting_to_activate.is_empty(),
            title: "Activate trigger effect?",
            BlockButton {
                label: "No",
                onclick: |_| send_user_response(UserResponse::No),
                additional_classes: "bg-red-600/70",
            }
        }
        MessageModal {
            trigger: (state.yes_no_question)().is_some(),
            title: (state.yes_no_question)().unwrap_or(String::new()),
            div {
                class: "flex flex-row gap-4",
                BlockButton {
                    label: "Yes",
                    onclick: |_| send_user_response(UserResponse::Yes),
                    additional_classes: "bg-green-600/70",
                }
                BlockButton {
                    label: "No",
                    onclick: |_| send_user_response(UserResponse::No),
                    additional_classes: "bg-red-600/70",
                }
            }
        }
        MessageModal {
            trigger: !state.positions_to_select.is_empty(),
            title: "Select battle position",
            {
                rsx!(
                    for position in (state.positions_to_select)() {
                        BlockButton {
                            label: position,
                            onclick: move |_| send_user_response(UserResponse::SelectPosition { position }),
                            additional_classes: "bg-gray-600 text-white",
                        }
                    }
                )
            }
        }

        CardSelector { }
        GraveyardModal {}

    )
}

#[component]
pub fn CardSelector() -> Element {
    let state = use_context::<DuelState>();
    let mut selected_card = use_signal(|| None);

    rsx!(
        CardPickerModal {
            title: "Select a card",
            trigger: !state.selectables.is_empty(),
            div {
                class: "flex flex-row gap-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for card in (state.selectables)() {
                    SelectableCard {
                        card_code: card.card_code,
                        value_to_set: card.sequence,
                        select_signal: selected_card
                    }
                }
            }
            BlockButton {
                label: "Done",
                disabled: !selected_card().is_some(),
                onclick: move |_| {
                    send_user_response(UserResponse::SelectCard { sequence: selected_card.unwrap() });
                    selected_card.set(None);
                },
                additional_classes: if selected_card().is_none() { "bg-gray-600 cursor-not-allowed" } else { "bg-green-700 cursor-pointer" },
            }
        }
    )
}

#[component]
pub fn GraveyardModal() -> Element {
    let state = use_context::<DuelState>();
    let graveyard = state.graveyard;
    let mut show_graveyard = state.show_graveyard;

    let mut selected_card = use_signal(|| None);

    rsx!(
        CardPickerModal {
            title: "Graveyard",
            trigger: show_graveyard(),
            div {
                class: "flex flex-row gap-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in graveyard().iter().enumerate() {
                    ActivatableCard {
                        index: index as u8,
                        card: *card,
                        select_signal: selected_card,
                    }
                }
            }
            BlockButton {
                label: "Close",
                onclick: move |_| {
                    show_graveyard.set(false);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}

#[component]
pub fn ActivatableCard(
    index: u8,
    card: Option<ActiveCard>,
    select_signal: WriteSignal<Option<u8>>,
) -> Element {
    let state = use_context::<DuelState>();
    let cards_prompting_to_activate = state.card_prompting_to_activate;

    let card = match card {
        Some(card) => card,
        None => return Err(anyhow!("lala").into()),
    };

    let prompted_card = cards_prompting_to_activate()
        .iter()
        .find(|card| card.location == CardLocation::Graveyard && card.sequence == index)
        .copied();

    let prompted = prompted_card.is_some();
    let chain_option = prompted_card.and_then(|card| card.chain_option);

    rsx!(
        div {
            class: "relative m-2 h-min",
            div {
                class: "absolute -inset-[2px] rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none -z-10",
                class: if !prompted {"hidden"},
            }
            SelectableCard {
                card_code: card.card_code,
                value_to_set: index,
                select_signal: select_signal
            }
            div {
                class: "absolute inset-1 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none animate-pulse z-20",
                class: if !prompted {"hidden"},
            }
            div {
                class: "absolute z-30 flex flex-col items-center justify-center w-min left-1/2 -translate-x-[50%] -translate-y-[128px] bg-black/60 px-8 py-1",
                class: if (select_signal() != Some(index)) || !prompted {"hidden"},
                style: "mask_image: linear-gradient(to right, transparent 0%, black 10%, black 90%, transparent 100%); -webkit-mask-image: linear-gradient(to right, transparent 0%, black 30%, black 70%, transparent 100%);",
                ActivateButton {
                    onclick: move |_| {
                        if prompted {
                            if let Some(chain_option) = chain_option {
                                send_user_response(UserResponse::Chain { sequence: chain_option });
                            } else {
                                send_user_response(UserResponse::Yes);
                            }

                            // if activatable {
                            //     send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
                            // }

                            select_signal.set(None);
                        }
                    }
                }
            }
        }
    )
}
