use dioxus::prelude::*;

use super::graveyard::Graveyard;
use super::svg::SummonIcon;
use crate::ocgcore::ActiveCard;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::SelectedCard;
use crate::state::send_user_response;
use crate::utility::CARD_BACK;
use crate::utility::EXTRA_BACK;

#[component]
pub fn Field() -> Element {
    let state = use_context::<DuelState>();
    let monsters = state.monsters;
    let spell_traps = state.spell_traps;

    rsx!(
        div { // Entire field
            class: "mx-auto flex flex-col gap-2 w-min pt-[15vh]",
            div { // Extra Monster Zones
                class: "flex flex-row gap-2 justify-center",
                div {
                    class: "size-[9vw] invisible",
                },
                div {
                    class: "size-[9vw] invisible",
                },
                div {
                    class: "",
                    Zone {index: 5, zone_type: CardLocation::MonsterZone}
                }
                div {
                    class: "size-[9vw] invisible",
                },
                div {
                    class: "",
                    Zone {index: 6, zone_type: CardLocation::MonsterZone}
                },
                div {
                    class: "size-[9vw] invisible",
                },
                div {
                    class: "size-[9vw] invisible",
                },
            }
            div { // Main Monster Zones + Field Zone + GY
                class: "flex flex-row gap-2 justify-center",
                Zone {index: 5, card: spell_traps().get(5).copied().flatten(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 0, card: monsters().first().copied().flatten(), zone_type: CardLocation::MonsterZone}
                Zone {index: 1, card: monsters().get(1).copied().flatten(), zone_type: CardLocation::MonsterZone}
                Zone {index: 2, card: monsters().get(2).copied().flatten(), zone_type: CardLocation::MonsterZone}
                Zone {index: 3, card: monsters().get(3).copied().flatten(), zone_type: CardLocation::MonsterZone}
                Zone {index: 4, card: monsters().get(4).copied().flatten(), zone_type: CardLocation::MonsterZone},
                Graveyard {}
            }
            div { // Spell/Trap Zones
                class: "flex flex-row gap-2 justify-center",
                ExtraDeck {}

                Zone {index: 0, card: spell_traps().first().copied().flatten(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 1, card: spell_traps().get(1).copied().flatten(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 2, card: spell_traps().get(2).copied().flatten(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 3, card: spell_traps().get(3).copied().flatten(), zone_type: CardLocation::SpellTrapZone}
                Zone {index: 4, card: spell_traps().get(4).copied().flatten(), zone_type: CardLocation::SpellTrapZone},

                MainDeck {}
            }
        }
    )
}

#[component]
fn Zone(index: u8, card: Option<ActiveCard>, zone_type: CardLocation) -> Element {
    let mut state = use_context::<DuelState>();
    let trigger_or_chain_effects = state.card_prompting_to_activate;
    let mut selected_card = state.selected_card;

    let activatable_effects = (state.activatable_effects)();
    let activatable_card = activatable_effects
        .iter()
        .find(|(_eff_index, card)| card.location == zone_type && card.sequence == index);
    let activatable_eff_index = activatable_card.map(|(i, _)| *i).unwrap_or(0);

    let prompted_card = trigger_or_chain_effects
        .iter()
        .find(|card| card.location == zone_type && card.sequence == index);
    let prompted = prompted_card.is_some();
    let activatable = activatable_card.is_some();
    let chain_option = prompted_card.and_then(|card| card.chain_option);

    let is_selected = selected_card()
        .is_some_and(|card| card.location == zone_type && card.index == index);

    let mut available_zones = state.available_zones;
    let clickable = available_zones()
        .iter()
        .any(|zone| zone.0 == zone_type && zone.1 == index);

    let effects_of_this_card: Vec<(u16, ActiveCard)> = activatable_effects
        .iter()
        .filter(|(_eff_index, card)| card.location == zone_type && card.sequence == index)
        .map(|(eff_index, card)| (*eff_index, card.clone()))
        .collect();

    rsx!(
        div {
            class: "shadow-xl bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center",
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
            if let Some(card) = card {
                div {
                    class: "relative h-full aspect-[59/86] mx-auto p-[clamp(1px,0.3vw,5px)]",
                    onclick: move |evt| {
                        evt.stop_propagation();

                        selected_card.set(Some(SelectedCard{
                            location: zone_type,
                            index: index as u8
                        }));
                    },
                    if activatable || prompted {
                        div {
                            class: "absolute inset-[clamp(1px,0.3vw,5px)] z-0 rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none"
                        }
                    }
                    img {
                        class: "relative z-10 w-full h-full object-contain",
                        class: if card.position == Some(BattlePosition::FaceDownDefense) || card.position == Some(BattlePosition::FaceUpDefense) {"-rotate-90"} else {""},
                        image_rendering: "smooth",
                        aspect_ratio: "59/86",
                        src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.card_code),
                    }
                    if activatable || prompted {
                        div {
                            class: "absolute inset-[clamp(1px,0.3vw,5px)] z-20 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none animate-pulse"
                        }
                    }

                    if is_selected {
                        if activatable || prompted {
                            div {
                                class: "absolute left-1/2 transform -translate-x-1/2 z-50 -top-18",
                                if activatable || prompted {
                                    div {
                                        class: "flex flex-col items-center justify-center gap-1 drop-shadow-xl/50",
                                        p {
                                            class: "text-white text-sm font-semibold shadow-md text-center ",
                                            "Activate"
                                        }
                                        button {
                                            class: "bg-black size-10 p-1 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer text-center",
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
                                                    if effects_of_this_card.len() > 1 {
                                                        state.effects_to_select_from.set(effects_of_this_card.clone());
                                                    } else {
                                                        send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
                                                    }
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

#[component]
fn MainDeck() -> Element {
    let state = use_context::<DuelState>();

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            for index in 1..(state.main_deck_length)() + 1 {
                div {
                    class: "absolute inset-[clamp(2px,0.6vw,8px)]",
                    img {
                        class: "w-full h-full object-contain",
                        style: "z-index: 10; transform: translate({index as f32 * 0.01}vw, -{index as f32 * 0.01}vh);",
                        image_rendering: "smooth",
                        aspect_ratio: "59/86",
                        src: CARD_BACK,
                    }
                }
            }
        }
    )
}

#[component]
fn ExtraDeck() -> Element {
    let state = use_context::<DuelState>();

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            for index in 1..(state.extra_deck_length)() + 1 {
                div {
                    class: "absolute inset-[clamp(2px,0.6vw,8px)]",
                    img {
                        class: "w-full h-full object-contain",
                        style: "z-index: 10; transform: translate({index as f32 * 0.01}vw, -{index as f32 * 0.01}vh);",
                        image_rendering: "smooth",
                        aspect_ratio: "59/86",
                        src: EXTRA_BACK,
                    }
                }
            }
        }
    )
}
