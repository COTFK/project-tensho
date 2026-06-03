use dioxus::prelude::*;

use super::components::ActionButton;
use super::components::Card;
use super::components::CardActionMenu;
use super::components::CardStack;
use super::components::OptionButton;
use super::components::PickerModal;
use super::constants::ZONE_SIZE;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_response;
use crate::ui::components::svg::SummonIcon;
use crate::utility::EXTRA_BACK;

#[component]
pub fn ExtraDeck() -> Element {
    let mut state = use_context::<DuelState>();
    let available_special_summons = (state.special_summons)();

    let has_summons = available_special_summons
        .iter()
        .any(|card| card.location == CardLocation::ExtraDeck);
    let has_cards = state.extra_deck.len() > 0;

    // Disable if effect selection modal is active
    let suppress_actions = !state.effects_to_select_from.is_empty();

    rsx!(
        div {
            class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5",
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards { state.show_extra_deck.set(true) },
            if has_cards {
                div {
                    class: "relative h-full aspect-[59/86]",
                    div {
                        class: "absolute inset-[1px] my-0.5 ml-0.5 mb-1 md:inset-[2px] md:my-1 md:mb-2 md:ml-1 rounded-[2px] blur-[1px] mix-blend-screen pointer-events-none",
                        class: if has_summons && !suppress_actions { "bg-yellow-400" },
                    }
                    CardStack {
                        length: state.extra_deck.len(),
                        image_url: EXTRA_BACK,
                    }
                }
            }
        }
    )
}

#[component]
pub fn ExtraDeckModal() -> Element {
    let state = use_context::<DuelState>();
    let extra_deck = state.extra_deck;
    let available_special_summons = (state.special_summons)();
    let mut show_extra_deck = state.show_extra_deck;

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Extra Deck",
            trigger: show_extra_deck(),
            div {
                class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in extra_deck().iter().enumerate() {
                    {
                        let card = (*card).unwrap();
                        let special_summon_index = available_special_summons
                            .iter()
                            .position(|summon| {
                                summon.location == CardLocation::ExtraDeck
                                    && summon.sequence == card.sequence
                            })
                            .map(|index| index as u8);
                        let is_special_summonable = special_summon_index.is_some();

                        // Disable if effect selection modal is active
                        let suppress_actions = !state.effects_to_select_from.is_empty();

                        rsx!(
                            div {
                                class: "relative py-2",
                                Card {
                                    card,
                                    class: "w-[12vw]",
                                    is_selected: selected_card() == Some(index),
                                    show_stats: false,
                                    show_highlight_on_select: true,
                                    show_dotted_highlight: false,
                                    show_blue_aura: false,
                                    show_orange_aura: is_special_summonable && !suppress_actions,
                                    facedown: false,
                                    use_extra_deck_back: false,
                                    onclick: move |_| selected_card.set(Some(index))
                                }
                                CardActionMenu {
                                    class: "absolute left-1/2 bottom-1/2 -translate-x-[50%] translate-y-[50%] px-3 py-2 md:px-6",
                                    trigger: selected_card() == Some(index) && is_special_summonable && !suppress_actions,
                                    ActionButton {
                                        label: "Summon",
                                        class: "border-yellow-500 text-yellow-300",
                                        onclick: move |_| {
                                            if let Some(special_summon_index) = special_summon_index {
                                                send_response(Response::SpecialSummon { index: special_summon_index });
                                            }
                                            selected_card.set(None);
                                            show_extra_deck.set(false);
                                        },
                                        SummonIcon {  }
                                    }
                                }
                            }
                        )
                    }

                }
            }
            OptionButton {
                label: "Close",
                onclick: move |_| {
                    show_extra_deck.set(false);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}
