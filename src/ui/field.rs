use dioxus::prelude::*;

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
            class: "mx-auto flex flex-col gap-2 w-min pt-[10vh]",
            div { // Extra Monster Zones
                class: "flex flex-row gap-2 justify-center",
                div {
                    class: "size-[10vw] invisible",
                },
                div {
                    class: "size-[10vw] invisible",
                },
                div {
                    class: "",
                    Zone {index: 5, zone_type: CardLocation::MonsterZone}
                }
                div {
                    class: "size-[10vw] invisible",
                },
                div {
                    class: "",
                    Zone {index: 6, zone_type: CardLocation::MonsterZone}
                },
                div {
                    class: "size-[10vw] invisible",
                },
                div {
                    class: "size-[10vw] invisible",
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
    let state = use_context::<DuelState>();
    let trigger_or_chain_effects = state.card_prompting_to_activate;
    let mut selected_card = state.selected_card;

    let activatable_effects = (state.activatable_effects)();
    let activatable_card = activatable_effects
        .iter()
        .find(|(_eff_index, card)| card.location == zone_type && card.sequence == index);
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
    let clickable = available_zones()
        .iter()
        .any(|zone| zone.0 == zone_type && zone.1 == index);

    rsx!(
        div {
            class: "shadow-xl bg-slate-50/2 size-[10vw] aspect-square flex items-center justify-center",
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
            if card.is_some() {
                div {
                    class: "relative",
                    onclick: move |evt| {
                        evt.stop_propagation();

                        if card.is_some() {
                            selected_card.set(Some(SelectedCard{
                                location: zone_type,
                                index: index as u8
                            }));
                        }
                    },
                    if activatable || prompted {
                        div {
                            class: "absolute inset-1 rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none"
                        }
                    }
                    img {
                        class: "relative h-[10vw] p-2",
                        class: if card.unwrap().position == Some(BattlePosition::FaceDownDefense) || card.unwrap().position == Some(BattlePosition::FaceUpDefense) {"-rotate-90"} else {""},
                        image_rendering: "smooth",
                        aspect_ratio: "59/86",
                        src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.unwrap().card_code),
                    }
                    if activatable || prompted {
                        div {
                            class: "absolute inset-2 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none animate-pulse"
                        }
                    }

                    if is_selected {
                        if activatable || prompted {
                            div {
                                class: "absolute -top-26 left-1/2 transform -translate-x-1/2 flex flex-col items-center justify-center z-50",
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

#[component]
fn Graveyard() -> Element {
    let mut state = use_context::<DuelState>();

    let any_trigger_effects_in_gy = state
        .card_prompting_to_activate
        .iter()
        .any(|card| card.location == CardLocation::Graveyard);

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[10vw] aspect-square flex items-center justify-center border-0.5 hover:border-4 hover:border-yellow-300",
            class: if any_trigger_effects_in_gy {"border-4 border-yellow-300/50"},
            onclick: move |_| state.show_graveyard.set(true),
            for (index, card) in (state.graveyard)().iter().enumerate() {
                img {
                    class: "absolute h-[10vw] p-2",
                    style: "transform: translate({index}px, -{index}px);",
                    image_rendering: "smooth",
                    aspect_ratio: "59/86",
                    src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.unwrap().card_code),
                }
            }
        }
    )
}

#[component]
fn MainDeck() -> Element {
    let state = use_context::<DuelState>();
    let count = state.duel.count_location(
        crate::ocgcore::constants::CardOwner::Player,
        CardLocation::Deck,
    );

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[10vw] aspect-square flex items-center justify-center border-0.5",
            for index in 0..count {
                img {
                    class: "absolute h-[10vw] p-2",
                    style: "z-index: 10; transform: translate({index as f32 * 0.5}px, -{index as f32 * 0.5}px);",
                    image_rendering: "smooth",
                    aspect_ratio: "59/86",
                    src: CARD_BACK,
                }
            }
        }
    )
}

#[component]
fn ExtraDeck() -> Element {
    let state = use_context::<DuelState>();
    let count = state.duel.count_location(
        crate::ocgcore::constants::CardOwner::Player,
        CardLocation::ExtraDeck,
    );

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[10vw] aspect-square flex items-center justify-center border-0.5",
            for index in 0..count {
                img {
                    class: "absolute h-[10vw] p-2",
                    style: "z-index: 10; transform: translate({index as f32 * 0.5}px, -{index as f32 * 0.5}px);",
                    image_rendering: "smooth",
                    aspect_ratio: "59/86",
                    src: EXTRA_BACK,
                }
            }
        }
    )
}
