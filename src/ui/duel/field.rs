use dioxus::prelude::*;

use super::card_stats::CardStats;
use super::constants::ZONE_SIZE;
use super::effect_pile::EffectPile;
use super::extra_deck::ExtraDeck;
use super::layout_anchors::{Anchor, AnchorPart, LayoutAnchor, LayoutRevision};
use super::main_deck::MainDeck;
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::state::{UIState, send_response};
use crate::ui::card_animation::CardAnimationState;

#[component]
pub fn Field() -> Element {
    let state = use_context::<UIState>();
    let revision = use_context::<LayoutRevision>();
    let monsters = state
        .zone_data(CardLocation::MonsterZone)
        .unwrap_or_default();
    let spell_traps = state
        .zone_data(CardLocation::SpellTrapZone)
        .unwrap_or_default();
    rsx!(
        div {
            class: "pt-[15vh] max-w-[90vw] w-fit mx-auto grid grid-cols-7 gap-0.5 justify-items-center",
            onresize: move |_| revision.invalidate(),
            div { class: "col-start-3 row-start-1", Zone { index: 5, card: monsters.get(5).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-1", Zone { index: 6, card: monsters.get(6).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-7 row-start-1 relative {ZONE_SIZE}", PileAnchor { location: CardLocation::Banishment } EffectPile { location: CardLocation::Banishment } }
            div { class: "col-start-1 row-start-2", Zone { index: 5, card: spell_traps.get(5).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-2 row-start-2", Zone { index: 0, card: monsters.first().copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-3 row-start-2", Zone { index: 1, card: monsters.get(1).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-4 row-start-2", Zone { index: 2, card: monsters.get(2).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-2", Zone { index: 3, card: monsters.get(3).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-6 row-start-2", Zone { index: 4, card: monsters.get(4).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-7 row-start-2 relative", PileAnchor { location: CardLocation::Graveyard } EffectPile { location: CardLocation::Graveyard } }
            div { class: "col-start-1 row-start-3 relative", PileAnchor { location: CardLocation::ExtraDeck } ExtraDeck {} }
            div { class: "col-start-2 row-start-3", Zone { index: 0, card: spell_traps.first().copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-3 row-start-3", Zone { index: 1, card: spell_traps.get(1).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-4 row-start-3", Zone { index: 2, card: spell_traps.get(2).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-5 row-start-3", Zone { index: 3, card: spell_traps.get(3).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-6 row-start-3", Zone { index: 4, card: spell_traps.get(4).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-7 row-start-3 relative", PileAnchor { location: CardLocation::Deck } MainDeck {} }
        }
    )
}

#[component]
fn Zone(index: u8, location: CardLocation, card: Option<CardData>) -> Element {
    let state = use_context::<UIState>();
    let animating = try_consume_context::<Signal<CardAnimationState>>()
        .is_some_and(|playback| playback().active);
    let placeable_on = (state.available_zones)()
        .iter()
        .any(|zone| zone.location == location && zone.sequence == index);
    rsx!(
        div {
            id: "{location}-{index}",
            class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center",
            class: if placeable_on {"border-2 border-yellow-300"} else {"border-0.5"},
            onclick: move |_| {
                if placeable_on {
                    send_response(Response::Place {
                        controller: Player::Player as u8,
                        location: location as u8,
                        index,
                    });
                }
            },
            LayoutAnchor {
                anchor: Anchor::Zone(location, u32::from(index)),
                part: AnchorPart::Card,
                class: "absolute h-full aspect-[59/86] p-[clamp(1px,0.3vw,5px)] pointer-events-none",
            }
            if let Some(card) = card {
                if !animating {
                    CardStats { card }
                }
            }

        }
    )
}

#[component]
fn PileAnchor(location: CardLocation) -> Element {
    rsx! {
        if location == CardLocation::Deck {
            div {
                class: "absolute inset-[clamp(2px,0.6vw,8px)] flex items-center justify-center pointer-events-none",
                LayoutAnchor {
                    anchor: Anchor::Pile(location),
                    part: AnchorPart::Card,
                    class: "h-full aspect-[59/86]",
                }
            }
        } else {
        div {
            class: "absolute left-1/2 top-1/2 h-full aspect-[59/86] -translate-x-1/2 -translate-y-1/2 pointer-events-none",
            div {
                class: "absolute inset-[clamp(2px,0.6vw,8px)] flex items-center justify-center",
                LayoutAnchor {
                    anchor: Anchor::Pile(location),
                    part: AnchorPart::Card,
                    class: "w-full aspect-[59/86]",
                }
                }
            }
        }
    }
}
