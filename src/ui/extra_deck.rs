use dioxus::prelude::*;

use super::components::ActionButton;
use super::components::Card;
use super::components::CardActionMenu;
use super::components::CardStack;
use super::components::OptionButton;
use super::components::PickerModal;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_user_response;
use crate::ui::components::svg::SummonIcon;
use crate::utility::EXTRA_BACK;

#[component]
pub fn ExtraDeck() -> Element {
    let mut state = use_context::<DuelState>();

    let has_cards = state.extra_deck.len() > 0;

    rsx!(
        div {
            class: "relative bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards { state.show_extra_deck.set(true) },
            CardStack {
                length: state.extra_deck.len(),
                image_url: EXTRA_BACK,
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
                class: "flex flex-row min-w-[40vw] w-full max-w-[77vw]",
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

                        rsx!(
                            div {
                                class: "relative p-[0.3vw]",
                                Card {
                                    code: card.card_code,
                                    class: "w-[8vw]",
                                    is_selected: selected_card() == Some(index),
                                    highlight_on_select: true,
                                    show_dotted_highlight: false,
                                    is_normal_summonable: false,
                                    is_activatable: is_special_summonable,
                                    onclick: move |_| selected_card.set(Some(index))
                                }
                                CardActionMenu {
                                    class: "absolute left-1/2 -translate-x-[50%] -translate-y-[96px]",
                                    trigger: selected_card() == Some(index) && is_special_summonable,
                                    ActionButton {
                                        label: "Summon",
                                        class: "border-yellow-500 text-yellow-300",
                                        onclick: move |_| {
                                            if let Some(special_summon_index) = special_summon_index {
                                                send_user_response(UserResponse::SpecialSummon { index: special_summon_index });
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
