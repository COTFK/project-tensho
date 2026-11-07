use dioxus::prelude::*;

use super::constants::ZONE_SIZE;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::state::UIState;
use crate::state::send_response;
use crate::ui::components::ActionButton;
use crate::ui::components::Card;
use crate::ui::components::CardActionMenu;
use crate::ui::components::CardImage;
use crate::ui::components::OptionButton;
use crate::ui::components::PickerModal;
use crate::ui::components::svg::LightningTrioIcon;

#[component]
pub fn EffectPile(location: CardLocation) -> Element {
    let mut state = use_context::<UIState>();

    let has_cards = state
        .zone_data(location)
        .is_some_and(|cards| !cards.is_empty());
    let has_trigger_effects = state
        .card_prompting_to_activate
        .iter()
        .any(|card| card.location == location);

    // Disable if effect selection modal is active
    let suppress_actions = !state.effects_to_select_from.is_empty();

    rsx!(
        div {
            class: if location == CardLocation::Banishment {
                "relative w-full h-full flex items-center justify-center border-0.5"
            } else {
                "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5"
            },
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards {
                state.open_pile.set(Some(location));
            },
            if has_cards {
                div {
                    class: if location == CardLocation::Banishment {
                        "relative h-full aspect-[59/86] -rotate-90"
                    } else {
                        "relative h-full aspect-[59/86]"
                    },
                    div {
                        class: "absolute inset-[2px] md:inset-[4px] my-0.5 md:my-1 rounded-[2px] blur-[1px] mix-blend-screen pointer-events-none",
                        class: if has_trigger_effects && !suppress_actions { "bg-yellow-400" },
                    }
                }
            }
        }
    )
}

#[component]
pub fn EffectPileModal(location: CardLocation) -> Element {
    let state = use_context::<UIState>();
    let cards = state.zone_data(location).unwrap_or_default();
    let cards_prompting_to_activate = state.card_prompting_to_activate;
    let mut open_pile = state.open_pile;
    let title = if location == CardLocation::Banishment {
        "Banishment"
    } else {
        "Graveyard"
    };

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title,
            trigger: open_pile() == Some(location),
            div {
                class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in cards.iter().enumerate() {
                    {
                        let prompted_card = cards_prompting_to_activate()
                            .iter()
                            .find(|card| card.location == location && card.sequence == index as u8)
                            .copied();
                        let chain_index = prompted_card.and_then(|card| card.action_index);

                        // Disable if effect selection modal is active
                        let suppress_actions = !state.effects_to_select_from.is_empty();

                        rsx!(
                            div {
                                class: "relative py-2",
                                Card {
                                    image: CardImage::Face(card.unwrap()),
                                    class: "w-[12vw]",
                                    is_selected: selected_card() == Some(index),
                                    show_highlight_on_select: true,
                                    show_dotted_highlight: false,
                                    show_blue_aura: false,
                                    show_orange_aura: prompted_card.is_some() && !suppress_actions,
                                    onclick: move |_| selected_card.set(Some(index))
                                }
                                CardActionMenu {
                                    class: "absolute left-1/2 bottom-1/2 -translate-x-[50%] translate-y-[50%] px-3 py-2 md:px-6",
                                    trigger: selected_card() == Some(index) && prompted_card.is_some() && !suppress_actions,
                                    ActionButton {
                                        label: "Activate",
                                        class: "border-yellow-500 text-yellow-300",
                                        onclick: move |_| {
                                            if prompted_card.is_some() && !suppress_actions {
                                                if let Some(index) = chain_index {
                                                    send_response(Response::Chain { index });
                                                } else {
                                                    send_response(Response::Yes);
                                                }

                                                selected_card.set(None);
                                            }
                                        },
                                        LightningTrioIcon {  }
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
                    open_pile.set(None);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}
