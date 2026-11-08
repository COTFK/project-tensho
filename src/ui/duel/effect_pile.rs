use crate::duel::input::PendingInput;
use dioxus::prelude::*;

use super::chain_badges::ChainBadges;
use super::constants::ZONE_SIZE;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::state::UIState;
use crate::state::send_response;
use crate::ui::card_layer::CardAddress;
use crate::ui::chain::ChainPlayback;
use crate::ui::components::ActionButton;
use crate::ui::components::Card;
use crate::ui::components::CardActionMenu;
use crate::ui::components::CardImage;
use crate::ui::components::OptionButton;
use crate::ui::components::PickerModal;
use crate::ui::components::svg::LightningTrioIcon;
use crate::ui::view_state::ViewState;

#[component]
pub fn EffectPile(location: CardLocation) -> Element {
    let state = use_context::<UIState>();
    let mut view = use_context::<ViewState>();

    let has_cards = state
        .zone_data(location)
        .is_some_and(|cards| !cards.is_empty());
    let has_trigger_effects = state.pending_input.read().as_ref().is_some_and(|input| {
        input
            .prompted_effects()
            .iter()
            .any(|card| card.location == location)
    });

    // Disable if effect selection modal is active
    let suppress_actions = !view.effects_to_select_from.is_empty();

    rsx!(
        div {
            class: if location == CardLocation::Banishment {
                "relative w-full h-full flex items-center justify-center border-0.5"
            } else {
                "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5"
            },
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards {
                view.open_pile.set(Some(location));
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
    let view = use_context::<ViewState>();
    let cards = state.zone_data(location).unwrap_or_default();
    let tracking = (state.cards)();
    let chain = try_consume_context::<Signal<ChainPlayback>>().map(|chain| chain());
    let pending = (state.pending_input)();
    let cards_prompting_to_activate = pending
        .as_ref()
        .map(PendingInput::prompted_effects)
        .unwrap_or_default();
    let mut open_pile = view.open_pile;
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
            retain_on_close: true,
            div {
                class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in cards.iter().enumerate() {
                    {
                        let prompted_card = cards_prompting_to_activate
                            .iter()
                            .find(|card| card.location == location && card.sequence == index as u8)
                            .copied();
                        let chain_index = prompted_card.and_then(|card| card.action_index);
                        let links = tracking.as_ref()
                            .and_then(|layer| layer.card_at(CardAddress {
                                controller: Player::Player,
                                location,
                                sequence: index as u32,
                            }))
                            .and_then(|card| chain.as_ref().map(|chain| chain.links_for_card(card.id)))
                            .unwrap_or_default();

                        // Disable if effect selection modal is active
                        let suppress_actions = !view.effects_to_select_from.is_empty();

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
                                ChainBadges { links }
                                CardActionMenu {
                                    class: "absolute left-1/2 bottom-1/2 -translate-x-[50%] translate-y-[50%] px-3 py-2 md:px-6 z-20",
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
