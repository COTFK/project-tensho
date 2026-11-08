use crate::duel::cards::{CardAddress, CardTracker};
use crate::duel::controller::ChainLink;
use crate::duel::input::{CardAction, PendingInput};
use dioxus::prelude::*;

use super::effect_pile::EffectPileModal;
use super::extra_deck::ExtraDeckModal;
use crate::ocgcore::constants::{BattlePosition, CardLocation, Player};
use crate::ocgcore::messages::{
    AnnounceNumberMessageData, SelectCardMessageData, SelectOptionMessageData,
    SelectTributeMessageData, SelectUnselectMessageData, SortCardMessageData,
};
use crate::ocgcore::{CardData, Response};
use crate::ui::components::Card;
use crate::ui::components::CardImage;
use crate::ui::components::MessageModal;
use crate::ui::components::OptionButton;
use crate::ui::components::PickerModal;
use crate::utility::get_optional_string_label;

#[component]
pub fn ModalContainer() -> Element {
    let pending = use_context::<ReadSignal<Option<PendingInput>>>()();
    let tracking = use_context::<ReadSignal<Option<CardTracker>>>();
    let chain = use_context::<Memo<Vec<ChainLink>>>()();
    let open_pile = use_context::<Signal<Option<CardLocation>>>();
    let effects = use_context::<Signal<Vec<CardData>>>();
    let onresponse = use_context::<EventHandler<Response>>();
    let onaction = use_context::<EventHandler<CardAction>>();
    let suppress_actions = !effects.read().is_empty();
    let prompts = pending
        .as_ref()
        .map(PendingInput::prompted_effects)
        .unwrap_or_default()
        .to_vec();
    let cards = tracking.read();
    rsx!(
        PromptActivationModal { prompts: prompts.clone(), onresponse }
        YesNoQuestionModal {
            question: match &pending { Some(PendingInput::YesNo(value)) => Some(value.clone()), _ => None },
            onresponse,
        }
        CardChoiceModal {
            cards_to_select_from: match &pending { Some(PendingInput::Unselect(value)) => Some(value.clone()), _ => None },
            tributes: match &pending { Some(PendingInput::Tribute { message, .. }) => Some(message.clone()), _ => None },
            selected_tributes: match &pending { Some(PendingInput::Tribute { selected, .. }) => selected.clone(), _ => Vec::new() },
            onresponse,
        }
        PositionSelectorModal {
            positions: match &pending { Some(PendingInput::Position(value)) => value.clone(), _ => Vec::new() },
            onresponse,
        }
        CardSelector {
            selectables: match &pending { Some(PendingInput::Cards(value)) => Some(value.clone()), _ => None },
            onresponse,
        }
        SortCardSelector {
            sort_cards: match &pending { Some(PendingInput::Sort(value)) => Some(value.clone()), _ => None },
            onresponse,
        }
        EffectPileModal {
            location: CardLocation::Graveyard,
            cards: pile_cards(cards.as_ref(), CardLocation::Graveyard, &chain),
            cards_prompting_to_activate: prompts.clone(),
            open_pile, suppress_actions, onaction,
        }
        EffectPileModal {
            location: CardLocation::Banishment,
            cards: pile_cards(cards.as_ref(), CardLocation::Banishment, &chain),
            cards_prompting_to_activate: prompts,
            open_pile, suppress_actions, onaction,
        }
        EffectSelector { effects, onresponse }
        ExtraDeckModal {
            extra_deck: pile_cards(cards.as_ref(), CardLocation::ExtraDeck, &chain),
            available_special_summons: match &pending { Some(PendingInput::Idle(value)) => value.special_summons.clone(), _ => Vec::new() },
            open_pile, suppress_actions, onaction,
        }
        OptionSelector {
            options_message: match &pending { Some(PendingInput::Options(value)) => Some(value.clone()), _ => None },
            onresponse,
        }
        NumberSelector {
            numbers: match &pending { Some(PendingInput::Numbers(value)) => Some(value.clone()), _ => None },
            onresponse,
        }
    )
}

#[derive(Clone, PartialEq)]
pub(super) struct PileCard {
    pub data: CardData,
    pub links: Vec<u32>,
}

fn pile_cards(
    tracking: Option<&CardTracker>,
    location: CardLocation,
    chain: &[ChainLink],
) -> Vec<PileCard> {
    let Some(tracking) = tracking else {
        return Vec::new();
    };
    tracking
        .zone_data(location)
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, data)| {
            let card = tracking.card_at(CardAddress {
                controller: Player::Player,
                location,
                sequence: index as u32,
            });
            PileCard {
                data: data.expect("Missing queried pile card data"),
                links: card
                    .map(|card| {
                        chain
                            .iter()
                            .filter_map(|link| (link.card == Some(card.id)).then_some(link.number))
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect()
}

#[component]
fn PromptActivationModal(prompts: Vec<CardData>, onresponse: EventHandler<Response>) -> Element {
    let has_action_index = prompts.iter().any(|card| card.action_index.is_some());

    rsx!(
        MessageModal {
            trigger: has_action_index,
            title: "A card or effect can be activated. Activate?",
            OptionButton {
                label: "No",
                onclick: move |_| onresponse.call(Response::PassPriority),
                additional_classes: "bg-red-600/70",
            }
        }
        MessageModal {
            trigger: !has_action_index && !prompts.is_empty(),
            title: "Activate trigger effect?",
            OptionButton {
                label: "No",
                onclick: move |_| onresponse.call(Response::No),
                additional_classes: "bg-red-600/70",
            }
        }
    )
}

#[component]
fn YesNoQuestionModal(question: Option<String>, onresponse: EventHandler<Response>) -> Element {
    rsx!(
        MessageModal {
            trigger: question.is_some(),
            title: question.unwrap_or_default(),
            div {
                class: "flex flex-row gap-4",
                OptionButton {
                    label: "Yes",
                    onclick: move |_| onresponse.call(Response::Yes),
                    additional_classes: "bg-green-600/70",
                }
                OptionButton {
                    label: "No",
                    onclick: move |_| onresponse.call(Response::No),
                    additional_classes: "bg-red-600/70",
                }
            }
        }
    )
}

#[component]
fn CardChoiceModal(
    cards_to_select_from: Option<SelectUnselectMessageData>,
    tributes: Option<SelectTributeMessageData>,
    selected_tributes: Vec<u8>,
    onresponse: EventHandler<Response>,
) -> Element {
    let selected_tributes_len = selected_tributes.len();
    let tribute_selection_is_valid = tributes.as_ref().is_some_and(|message| {
        selected_tributes_len >= message.min_select as usize
            && selected_tributes_len <= message.max_select as usize
    });

    rsx!(
        MessageModal {
            trigger: cards_to_select_from.is_some() || tributes.is_some(),
            title: "Select cards",
            if let Some(message) = cards_to_select_from {
                div {
                    class: "flex flex-row gap-4",
                    OptionButton {
                        label: "Confirm",
                        disabled: !message.finishable,
                        onclick: move |_| onresponse.call(Response::PassPriority),
                        additional_classes: "bg-green-600/70",
                    }
                    OptionButton {
                        label: "Cancel",
                        disabled: !message.cancelable,
                        onclick: move |_| onresponse.call(Response::PassPriority),
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
                        onclick: move |_| onresponse.call(Response::SelectTributes { tributes: selected_tributes.clone() }),
                    }
                    OptionButton {
                        label: "Cancel",
                        disabled: !message.is_cancelable,
                        onclick: move |_| onresponse.call(Response::PassPriority),
                        additional_classes: "bg-red-600/70",
                    }
                }
            }
        }
    )
}

#[component]
fn PositionSelectorModal(
    positions: Vec<BattlePosition>,
    onresponse: EventHandler<Response>,
) -> Element {
    rsx!(
        MessageModal {
            trigger: !positions.is_empty(),
            title: "Select battle position",
            for position in positions {
                OptionButton {
                    label: position,
                    onclick: move |_| onresponse.call(Response::SelectPosition { position }),
                    additional_classes: "bg-gray-600 text-white",
                }
            }
        }
    )
}

#[component]
pub fn CardSelector(
    selectables: Option<SelectCardMessageData>,
    onresponse: EventHandler<Response>,
) -> Element {
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
            retain_on_close: true,
            if let Some(message) = selectables {
                div {
                    class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2 pb-2",
                    class: "overflow-x-auto scroll-smooth scrollbar-thin",
                    for (index, card) in message.cards.iter().enumerate() {
                        div {
                            class: "relative",
                            Card {
                                image: CardImage::Face(*card),
                                class: "w-[12vw] min-w-[12vw]",
                                is_selected: selected_cards().contains(&(index as u8)),
                                show_highlight_on_select: true,
                                show_dotted_highlight: false,
                                show_blue_aura: false,
                                show_orange_aura: false,
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
                            div {
                                class: "absolute bottom-0 right-0 text-white text-[8px] md:text-xs xl:text-base font-semibold z-5 bg-gray-900/75 pl-1 pr-0.5 pt-0.5 rounded-tl-lg",
                                {format!("{} [{}]", card.location, card.sequence)}
                            }
                        }
                    }
                }
                OptionButton {
                    label: "Done",
                    disabled: !can_confirm,
                    onclick: move |_| {
                        onresponse.call(Response::SelectCard { indices: selected_cards() });
                        selected_cards.set(Vec::new());
                    },
                    additional_classes: if can_confirm { "bg-green-700 cursor-pointer" } else { "bg-gray-600 cursor-not-allowed" },
                }
            }
        }
    )
}

#[component]
pub fn SortCardSelector(
    sort_cards: Option<SortCardMessageData>,
    onresponse: EventHandler<Response>,
) -> Element {
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
            retain_on_close: true,
            if !cards.is_empty() {
                div {
                    class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                    class: "overflow-x-auto scroll-smooth scrollbar-thin",
                    for (index, card) in cards.iter().enumerate() {
                        div {
                            class: "relative w-[12vw] min-w-[12vw]",
                            Card {
                                image: CardImage::Face(*card),
                                class: "w-full",
                                is_selected: selected_cards().iter().position(|selected| *selected == index as u8).is_some(),
                                show_highlight_on_select: true,
                                show_dotted_highlight: false,
                                show_blue_aura: false,
                                show_orange_aura: false,
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
                        onresponse.call(Response::SortCard { indices: selected_cards() });
                    },
                    additional_classes: if can_confirm { "bg-green-700 cursor-pointer" } else { "bg-gray-600 cursor-not-allowed" },
                }
            }
        }
    )
}

#[component]
pub fn EffectSelector(
    mut effects: Signal<Vec<CardData>>,
    onresponse: EventHandler<Response>,
) -> Element {
    rsx!(
        PickerModal {
            title: "Choose which effect to activate",
            trigger: !effects.is_empty(),
            for effect in effects() {
                OptionButton {
                    label: {
                        effect
                            .description
                            .map(|description| get_optional_string_label(effect.card_code, description as usize))
                            .unwrap_or_else(|| String::from("error"))
                    },
                    onclick: move |_| onresponse.call(Response::Activate { index: effect.action_index.unwrap() }),
                    additional_classes: "bg-gray-600 text-white w-full",
                }
            }
            OptionButton {
                label: "Cancel",
                onclick: move |_| effects.clear(),
                additional_classes: "bg-gray-600 text-white w-full",
            }

        }
    )
}

#[component]
pub fn OptionSelector(
    options_message: Option<SelectOptionMessageData>,
    onresponse: EventHandler<Response>,
) -> Element {
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
                        onclick: move |_| onresponse.call(Response::SelectOption { index: index as u8 }),
                        additional_classes: "bg-gray-600 text-white w-full",
                    }
                }
            }
        }
    )
}

#[component]
pub fn NumberSelector(
    numbers: Option<AnnounceNumberMessageData>,
    onresponse: EventHandler<Response>,
) -> Element {
    rsx!(
        PickerModal {
            title: "Choose an option",
            trigger: numbers.is_some(),
            if let Some(message) = numbers {
                for (index, number) in message.numbers.iter().enumerate() {
                    OptionButton {
                        label: "{number}",
                        onclick: move |_| onresponse.call(Response::SelectOption { index: index as u8 }),
                        additional_classes: "bg-gray-600 text-white w-full",
                    }
                }
            }
        }
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/card_modals.rs"]
mod tests;
