use dioxus::prelude::*;

use super::card::Card;
use super::svg::SummonIcon;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::CardController;
use crate::state::DuelState;
use crate::state::SelectedCard;
use crate::state::send_user_response;

#[component]
pub fn Field() -> Element {
    let state = use_context::<DuelState>();
    let monsters = state.monsters;
    let spell_traps = state.spell_traps;

    rsx!(
        div { // Entire field
            class: "mx-auto flex flex-col gap-3 w-min pt-24",
            div { // Extra Monster Zones
                class: "flex flex-row gap-3 justify-center",
                div {
                    class: "size-[12vw] invisible",
                },
                div {
                    class: "size-[12vw] invisible",
                },
                div {
                    class: "",
                    Zone {index: 5, zone_type: CardLocation::MonsterZone}
                }
                div {
                    class: "size-[12vw] invisible",
                },
                div {
                    class: "",
                    Zone {index: 6, zone_type: CardLocation::MonsterZone}
                },
                div {
                    class: "size-[12vw] invisible",
                },
                div {
                    class: "size-[12vw] invisible",
                },
            }
            div { // Main Monster Zones + Field Zone + GY
                class: "flex flex-row gap-3 justify-center",
                Zone {index: 5, id: spell_traps().get(5).copied(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 0, id: monsters().first().copied(), zone_type: CardLocation::MonsterZone}
                Zone {index: 1, id: monsters().get(1).copied(), zone_type: CardLocation::MonsterZone}
                Zone {index: 2, id: monsters().get(2).copied(), zone_type: CardLocation::MonsterZone}
                Zone {index: 3, id: monsters().get(3).copied(), zone_type: CardLocation::MonsterZone}
                Zone {index: 4, id: monsters().get(4).copied(), zone_type: CardLocation::MonsterZone},
                div {
                    class: "size-[12vw] invisible",
                },
            }
            div { // Spell/Trap Zones
                class: "flex flex-row gap-3 justify-center",
                div {
                    class: "size-[12vw] invisible",
                },
                Zone {index: 0, id: spell_traps().first().copied(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 1, id: spell_traps().get(1).copied(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 2, id: spell_traps().get(2).copied(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 3, id: spell_traps().get(3).copied(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 4, id: spell_traps().get(4).copied(), zone_type: CardLocation::SpellTrapZone},
                div {
                    class: "size-[12vw] invisible",
                }
            }
        }
    )
}

#[component]
fn Zone(index: u8, id: Option<u32>, zone_type: CardLocation) -> Element {
    let state = use_context::<DuelState>();
    let trigger_or_chain_effects = state.card_prompting_to_activate;
    let mut selected_card = state.selected_card;

    let activatable_effects = (state.activatable_effects)();
    let activatable_card = activatable_effects.iter().find(|(eff_index, card)| card.location == zone_type && card.sequence == index);
    let activatable_eff_index = if activatable_card.is_some() {
        *activatable_card.unwrap().0
    } else {
        0
    };

    let prompted_card = trigger_or_chain_effects
        .iter()
        .find(|card| card.location == zone_type && card.sequence == index);
    let prompted = prompted_card.is_some();
    let activatable = activatable_card.is_some();
    let chain_option = prompted_card.and_then(|card| card.chain_option);

    let is_selected =
        selected_card().is_some_and(|card| card.location == zone_type && card.index == index);

    let mut available_zones = state.available_zones;
    let clickable = available_zones().iter().any(|zone| zone.0 == zone_type && zone.1 == index);

    rsx!(
        div {
            class: "shadow-xl bg-slate-50/2 size-[12vw] aspect-square flex items-center justify-center",
            class: if clickable {"border-2 border-yellow-300"} else {"border-0.5"},
            onclick: move |_| {
                if clickable {
                    send_user_response(UserResponse::Place {
                        controller: CardController::Player as u8,
                        location: zone_type as u8,
                        index: index as u8,
                    });

                    available_zones.clear();
                }
            },
            if id != Some(0) && id.is_some() {
                div {
                    class: "relative",
                    onclick: move |evt| {
                        evt.stop_propagation();

                        if id.is_some() {
                            selected_card.set(Some(SelectedCard{
                                location: zone_type,
                                index: index as u8
                            }));
                        }
                    },
                    if activatable || prompted {
                        div {
                            class: "absolute -inset-[5px] rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none z-10"
                        }
                    }
                    div {
                        class: "relative z-30",
                        Card {
                            id: id.unwrap()
                        }
                    }
                    if activatable || prompted{
                        div {
                            class: "absolute inset-0 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none z-30 animate-pulse"
                        }
                    }

                    if is_selected {
                        if activatable || prompted {
                            div {
                                class: "absolute -top-26 left-1/2 transform -translate-x-1/2 z-50 flex flex-col items-center justify-center",
                                if activatable || prompted {
                                    div {
                                        p {
                                            class: "text-white font-semibold shadow-md",
                                            "Activate"
                                        }
                                        button {
                                            class: "bg-black size-16 p-2 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer",
                                            onclick: move |evt| {
                                                evt.stop_propagation();

                                                if prompted {
                                                    if let Some(chain_option) = chain_option {
                                                        send_user_response(UserResponse::Chain { sequence: chain_option });
                                                    } else {
                                                        send_user_response(UserResponse::Yes);
                                                    }
                                                }

                                                if activatable {
                                                    send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
                                                }
                                            },
                                            SummonIcon {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    )
}
