use dioxus::prelude::*;

use super::components::Card;
use super::components::MessageModal;
use super::components::OptionButton;
use super::components::PickerModal;
use super::extra_deck::ExtraDeckModal;
use super::graveyard::GraveyardModal;
use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::send_user_response;
use crate::utility::get_cached_label;

#[component]
pub fn ModalContainer() -> Element {
    let state = use_context::<DuelState>();
    let cards_to_select_from = (state.cards_to_select_from)();

    rsx!(
        MessageModal {
            trigger: state.card_prompting_to_activate.iter().any(|card| card.chain_option.is_some()),
            title: "A card or effect can be activated. Activate?",
            OptionButton {
                label: "No",
                onclick: |_| send_user_response(UserResponse::PassPriority),
                additional_classes: "bg-red-600/70",
            }
        }
        MessageModal {
            trigger: !state.card_prompting_to_activate.iter().any(|card| card.chain_option.is_some()) && !state.card_prompting_to_activate.is_empty(),
            title: "Activate trigger effect?",
            OptionButton {
                label: "No",
                onclick: |_| send_user_response(UserResponse::No),
                additional_classes: "bg-red-600/70",
            }
        }
        MessageModal {
            trigger: (state.yes_no_question)().is_some(),
            title: (state.yes_no_question)().unwrap_or_default(),
            div {
                class: "flex flex-row gap-4",
                OptionButton {
                    label: "Yes",
                    onclick: |_| send_user_response(UserResponse::Yes),
                    additional_classes: "bg-green-600/70",
                }
                OptionButton {
                    label: "No",
                    onclick: |_| send_user_response(UserResponse::No),
                    additional_classes: "bg-red-600/70",
                }
            }
        }
        MessageModal {
            trigger: cards_to_select_from.is_some(),
            title: "Select cards",
            if let Some(selection_message) = cards_to_select_from {
                div {
                    class: "flex flex-row gap-4",
                    OptionButton {
                        label: "Confirm",
                        disabled: !selection_message.finishable,
                        onclick: |_| send_user_response(UserResponse::PassPriority),
                        additional_classes: "bg-green-600/70",
                    }
                    OptionButton {
                        label: "Cancel",
                        disabled: !selection_message.cancelable,
                        onclick: |_| send_user_response(UserResponse::PassPriority),
                        additional_classes: "bg-red-600/70",
                    }
                }
            }
        }
        MessageModal {
            trigger: !state.positions_to_select.is_empty(),
            title: "Select battle position",
            for position in (state.positions_to_select)() {
                OptionButton {
                    label: position,
                    onclick: move |_| send_user_response(UserResponse::SelectPosition { position }),
                    additional_classes: "bg-gray-600 text-white",
                }
            }
        }

        CardSelector { }
        GraveyardModal {}
        ExtraDeckModal {}
        EffectSelector {}

    )
}

#[component]
pub fn CardSelector() -> Element {
    let state = use_context::<DuelState>();
    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Select a card",
            trigger: !state.selectables.is_empty(),
            div {
                class: "flex flex-row gap-2 py-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin max-w-[80vw]",
                for (index, card) in (state.selectables)().iter().enumerate() {
                    Card {
                        code: card.card_code,
                        class: "w-[12vw] max-h-[40vh] min-w-[12vw]",
                        is_selected: selected_card() == Some(index as u8),
                        highlight_on_select: true,
                        show_dotted_highlight: false,
                        is_normal_summonable: false,
                        is_activatable: false,
                        onclick: move |_| selected_card.set(Some(index as u8))
                    }
                }
            }
            OptionButton {
                label: "Done",
                disabled: selected_card().is_none(),
                onclick: move |_| {
                    send_user_response(UserResponse::SelectCard { index: selected_card.unwrap() });
                    selected_card.set(None);
                },
                additional_classes: if selected_card().is_none() { "bg-gray-600 cursor-not-allowed" } else { "bg-green-700 cursor-pointer" },
            }
        }
    )
}

#[component]
pub fn EffectSelector() -> Element {
    let mut state = use_context::<DuelState>();

    rsx!(
        PickerModal {
            title: "Choose which effect to activate",
            trigger: !state.effects_to_select_from.is_empty(),
            for (index, effect) in (state.effects_to_select_from)() {
                OptionButton {
                    label: {
                        get_cached_label(effect.card_code)
                            .and_then(|card_label| {
                                effect.description.and_then(|description| {
                                    card_label.optional_strings.get(&(description as usize)).cloned()
                                })
                            })
                            .unwrap_or_else(|| String::from("error"))
                    },
                    onclick: move |_| send_user_response(UserResponse::Activate { index: index as u8 }),
                    additional_classes: "bg-gray-600 text-white w-full",
                }
            }
            OptionButton {
                label: "Cancel",
                onclick: move |_| state.effects_to_select_from.clear(),
                additional_classes: "bg-gray-600 text-white w-full",
            }

        }
    )
}
