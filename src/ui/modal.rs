use dioxus::prelude::*;

use super::components::Card;
use super::components::MessageModal;
use super::components::OptionButton;
use super::components::PickerModal;
use super::extra_deck::ExtraDeckModal;
use super::graveyard::GraveyardModal;
use crate::ocgcore::Response;
use crate::state::DuelState;
use crate::state::send_response;
use crate::utility::get_optional_string_label;

#[component]
pub fn ModalContainer() -> Element {
    let state = use_context::<DuelState>();
    let cards_to_select_from = (state.cards_to_select_from)();
    let tributes = (state.tributes)();
    let selected_tributes = (state.selected_tributes)();
    let selected_tributes_len = selected_tributes.len();
    let tribute_selection_is_valid = tributes.as_ref().is_some_and(|message| {
        selected_tributes_len >= message.min_select as usize
            && selected_tributes_len <= message.max_select as usize
    });

    rsx!(
        MessageModal {
            trigger: state.card_prompting_to_activate.iter().any(|card| card.action_index.is_some()),
            title: "A card or effect can be activated. Activate?",
            OptionButton {
                label: "No",
                onclick: |_| send_response(Response::PassPriority),
                additional_classes: "bg-red-600/70",
            }
        }
        MessageModal {
            trigger: !state.card_prompting_to_activate.iter().any(|card| card.action_index.is_some()) && !state.card_prompting_to_activate.is_empty(),
            title: "Activate trigger effect?",
            OptionButton {
                label: "No",
                onclick: |_| send_response(Response::No),
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
                    onclick: |_| send_response(Response::Yes),
                    additional_classes: "bg-green-600/70",
                }
                OptionButton {
                    label: "No",
                    onclick: |_| send_response(Response::No),
                    additional_classes: "bg-red-600/70",
                }
            }
        }
        MessageModal {
            trigger: cards_to_select_from.is_some() || tributes.is_some(),
            title: "Select cards",
            if let Some(message) = cards_to_select_from {
                div {
                    class: "flex flex-row gap-4",
                    OptionButton {
                        label: "Confirm",
                        disabled: !message.finishable,
                        onclick: |_| send_response(Response::PassPriority),
                        additional_classes: "bg-green-600/70",
                    }
                    OptionButton {
                        label: "Cancel",
                        disabled: !message.cancelable,
                        onclick: |_| send_response(Response::PassPriority),
                        additional_classes: "bg-red-600/70",
                    }
                }
            }
            if let Some(message) = tributes {
                div {
                    class: "flex flex-row gap-4",
                    OptionButton {
                        label: "Confirm",
                        disabled: !tribute_selection_is_valid,
                        additional_classes: if tribute_selection_is_valid { "bg-green-700 cursor-pointer" } else { "bg-gray-600 cursor-not-allowed" },
                        onclick: move |_| send_response(Response::SelectTributes { tributes: selected_tributes.clone() }),
                    }
                    OptionButton {
                        label: "Cancel",
                        disabled: !message.is_cancelable,
                        onclick: |_| send_response(Response::PassPriority),
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
                    onclick: move |_| send_response(Response::SelectPosition { position }),
                    additional_classes: "bg-gray-600 text-white",
                }
            }
        }

        CardSelector { }
        SortCardSelector { }
        GraveyardModal {}
        EffectSelector {}
        ExtraDeckModal {}
        OptionSelector {}
        NumberSelector {}

    )
}

#[component]
pub fn CardSelector() -> Element {
    let state = use_context::<DuelState>();
    let selectables = (state.selectables)();
    let mut selected_cards = use_signal(Vec::new);

    let selected_count = selected_cards().len();
    let can_confirm = selectables.as_ref().is_some_and(|message| {
        selected_count >= message.min_select as usize
            && selected_count <= message.max_select as usize
    });
    let title = selectables.as_ref().map(|message| {
        if message.max_select == 1 {
            String::from("Select a card")
        } else if message.min_select == message.max_select {
            format!("Select {} cards", message.min_select)
        } else {
            format!("Select up to {} cards", message.max_select)
        }
    });

    rsx!(
        PickerModal {
            title: title.unwrap_or("Select cards".to_string()),
            trigger: selectables.as_ref().is_some_and(|message| !message.cards.is_empty()),
            if let Some(message) = selectables {
                div {
                    class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                    class: "overflow-x-auto scroll-smooth scrollbar-thin",
                    for (index, card) in message.cards.iter().enumerate() {
                        Card {
                            code: card.card_code,
                            class: "w-[12vw] min-w-[12vw]",
                            is_selected: selected_cards().contains(&(index as u8)),
                            show_highlight_on_select: true,
                            show_dotted_highlight: false,
                            show_blue_aura: false,
                            show_orange_aura: false,
                            facedown: false,
                            use_extra_deck_back: false,
                            onclick: move |_| {
                                selected_cards.with_mut(|indices| {
                                    if message.max_select == 1 {
                                        *indices = vec![index as u8];
                                        return;
                                    }

                                    if let Some(position) = indices.iter().position(|selected| *selected == index as u8) {
                                        indices.remove(position);
                                    } else if indices.len() < message.max_select as usize {
                                        indices.push(index as u8);
                                    }
                                });
                            }
                        }
                    }
                }
                OptionButton {
                    label: "Done",
                    disabled: !can_confirm,
                    onclick: move |_| {
                        send_response(Response::SelectCard { indices: selected_cards() });
                        selected_cards.set(Vec::new());
                    },
                    additional_classes: if can_confirm { "bg-green-700 cursor-pointer" } else { "bg-gray-600 cursor-not-allowed" },
                }
            }
        }
    )
}

#[component]
pub fn SortCardSelector() -> Element {
    let state = use_context::<DuelState>();
    let sort_cards = (state.sort_cards_to_select_from)();
    let cards = sort_cards
        .as_ref()
        .map(|message| message.cards.clone())
        .unwrap_or_default();
    let mut selected_cards = use_signal(Vec::new);

    let selected_count = selected_cards().len();
    let cards_len = cards.len();
    let can_confirm = !cards.is_empty() && selected_count == cards_len;

    rsx!(
        PickerModal {
            title: "Sort cards",
            trigger: !cards.is_empty(),
            if !cards.is_empty() {
                div {
                    class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                    class: "overflow-x-auto scroll-smooth scrollbar-thin",
                    for (index, card) in cards.iter().enumerate() {
                        div {
                            class: "relative w-[12vw] min-w-[12vw]",
                            Card {
                                code: card.card_code,
                                class: "w-full",
                                is_selected: selected_cards().iter().position(|selected| *selected == index as u8).is_some(),
                                show_highlight_on_select: true,
                                show_dotted_highlight: false,
                                show_blue_aura: false,
                                show_orange_aura: false,
                                facedown: false,
                                use_extra_deck_back: false,
                                onclick: move |_| {
                                    selected_cards.with_mut(|indices| {
                                        if let Some(position) = indices.iter().position(|selected| *selected == index as u8) {
                                            indices.remove(position);
                                        } else if indices.len() < cards_len {
                                            indices.push(index as u8);
                                        }
                                    });
                                },
                            }
                            if let Some(order) = selected_cards().iter().position(|selected| *selected == index as u8) {
                                div {
                                    class: "pointer-events-none absolute left-2 top-2 z-20 flex h-7 w-7 items-center justify-center rounded-full bg-black/80 text-sm font-bold text-white ring-2 ring-white/80",
                                    "{order + 1}"
                                }
                            }
                        }
                    }
                }
                OptionButton {
                    label: "Done",
                    disabled: !can_confirm,
                    onclick: move |_| {
                        send_response(Response::SortCard { indices: selected_cards() });
                    },
                    additional_classes: if can_confirm { "bg-green-700 cursor-pointer" } else { "bg-gray-600 cursor-not-allowed" },
                }
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
            for effect in (state.effects_to_select_from)() {
                OptionButton {
                    label: {
                        effect
                            .description
                            .map(|description| get_optional_string_label(effect.card_code, description as usize))
                            .unwrap_or_else(|| String::from("error"))
                    },
                    onclick: move |_| send_response(Response::Activate { index: effect.action_index.unwrap() }),
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

#[component]
pub fn OptionSelector() -> Element {
    let state = use_context::<DuelState>();
    let options_message = (state.options_to_prompt)();

    rsx!(
        PickerModal {
            title: "Choose an option",
            trigger: options_message.as_ref().is_some_and(|message| !message.options.is_empty()),
            if let Some(message) = options_message {
                for (index, option) in message.options.iter().enumerate() {
                    OptionButton {
                        label: {
                            get_optional_string_label(option.card_code.unwrap(), option.string_index.unwrap())
                        },
                        onclick: move |_| send_response(Response::SelectOption { index: index as u8 }),
                        additional_classes: "bg-gray-600 text-white w-full",
                    }
                }
            }
        }
    )
}

#[component]
pub fn NumberSelector() -> Element {
    let state = use_context::<DuelState>();
    let numbers = (state.numbers_to_select_from)();

    rsx!(
        PickerModal {
            title: "Choose an option",
            trigger: numbers.is_some(),
            if let Some(message) = numbers {
                for (index, number) in message.numbers.iter().enumerate() {
                    OptionButton {
                        label: "{number}",
                        onclick: move |_| send_response(Response::SelectOption { index: index as u8 }),
                        additional_classes: "bg-gray-600 text-white w-full",
                    }
                }
            }
        }
    )
}
