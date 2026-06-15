use dioxus::prelude::*;

use super::components::ActionButton;
use super::components::Card;
use super::components::CardActionMenu;
use super::components::OptionButton;
use super::components::PickerModal;
use super::components::svg::SummonIcon;
use super::constants::ZONE_SIZE;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_response;
use crate::ui::components::CardStack;

#[component]
pub fn Banishment() -> Element {
    let mut state = use_context::<DuelState>();

    let has_cards = state.banishment.len() > 0;
    let has_trigger_effects = state
        .card_prompting_to_activate
        .iter()
        .any(|card| card.location == CardLocation::Banishment);

    // Disable if effect selection modal is active
    let suppress_actions = !state.effects_to_select_from.is_empty();

    rsx!(
        div {
            class: "relative flex items-center justify-center border-0.5",
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards { state.show_banishment.set(true) },
            if has_cards {
                div {
                    class: "relative {ZONE_SIZE} aspect-[59/86] -rotate-90",
                    div {
                        class: "absolute inset-[2px] md:inset-[4px] my-0.5 md:my-1 rounded-[2px] blur-[1px] mix-blend-screen pointer-events-none",
                        class: if has_trigger_effects && !suppress_actions { "bg-yellow-400" },
                    }
                    CardStack {
                        length: state.banishment.len(),
                        image_url: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", (state.banishment)().last().unwrap().unwrap().card_code),
                    }
                }
            }
        }
    )
}

#[component]
pub fn BanishmentModal() -> Element {
    let state = use_context::<DuelState>();
    let banishment = state.banishment;
    let cards_prompting_to_activate = state.card_prompting_to_activate;
    let mut show_banishment = state.show_banishment;

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Banishment",
            trigger: show_banishment(),
            div {
                class: "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in banishment().iter().enumerate() {
                    {
                        let prompted_card = cards_prompting_to_activate()
                            .iter()
                            .find(|card| card.location == CardLocation::Banishment && card.sequence == index as u8)
                            .copied();
                        let chain_index = prompted_card.and_then(|card| card.action_index);

                        // Disable if effect selection modal is active
                        let suppress_actions = !state.effects_to_select_from.is_empty();

                        rsx!(
                            div {
                                class: "relative py-2",
                                Card {
                                    card: card.unwrap(),
                                    class: "w-[12vw]",
                                    is_selected: selected_card() == Some(index),
                                    show_highlight_on_select: true,
                                    show_dotted_highlight: false,
                                    show_blue_aura: false,
                                    show_orange_aura: prompted_card.is_some() && !suppress_actions,
                                    use_extra_deck_back: false,
                                    facedown: false,
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

                                                // if activatable {
                                                //     send_user_response(UserResponse::Activate { index: activatable_eff_index as u8 });
                                                // }

                                                selected_card.set(None);
                                            }
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
                    show_banishment.set(false);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}
