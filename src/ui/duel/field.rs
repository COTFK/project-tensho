use dioxus::prelude::*;

use super::banishment::Banishment;
use super::card_interaction::InteractiveCard;
use super::card_stats::CardStats;
use super::constants::ZONE_SIZE;
use super::extra_deck::ExtraDeck;
use super::graveyard::Graveyard;
use super::layout_anchors::{Anchor, AnchorPart, LayoutAnchor, LayoutRevision};
use super::main_deck::MainDeck;
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::state::{UIState, send_response};
use crate::ui::card_layer::CardAddress;

#[component]
pub fn Field(layered: bool) -> Element {
    let state = use_context::<UIState>();
    let revision = use_context::<LayoutRevision>();
    let monsters = (state.monsters)();
    let spell_traps = (state.spell_traps)();
    rsx!(
        div {
            class: "pt-[15vh] max-w-[90vw] w-fit mx-auto grid grid-cols-7 gap-0.5 justify-items-center",
            onresize: move |_| revision.invalidate(),
            div { class: "col-start-3 row-start-1", Zone { index: 5, card: monsters.get(5).copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-5 row-start-1", Zone { index: 6, card: monsters.get(6).copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-7 row-start-1 relative", PileAnchor { location: CardLocation::Banishment } Banishment {} }
            div { class: "col-start-1 row-start-2", Zone { index: 5, card: spell_traps.get(5).copied().flatten(), location: CardLocation::SpellTrapZone, layered } }
            div { class: "col-start-2 row-start-2", Zone { index: 0, card: monsters.first().copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-3 row-start-2", Zone { index: 1, card: monsters.get(1).copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-4 row-start-2", Zone { index: 2, card: monsters.get(2).copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-5 row-start-2", Zone { index: 3, card: monsters.get(3).copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-6 row-start-2", Zone { index: 4, card: monsters.get(4).copied().flatten(), location: CardLocation::MonsterZone, layered } }
            div { class: "col-start-7 row-start-2 relative", PileAnchor { location: CardLocation::Graveyard } Graveyard {} }
            div { class: "col-start-1 row-start-3 relative", PileAnchor { location: CardLocation::ExtraDeck } ExtraDeck {} }
            div { class: "col-start-2 row-start-3", Zone { index: 0, card: spell_traps.first().copied().flatten(), location: CardLocation::SpellTrapZone, layered } }
            div { class: "col-start-3 row-start-3", Zone { index: 1, card: spell_traps.get(1).copied().flatten(), location: CardLocation::SpellTrapZone, layered } }
            div { class: "col-start-4 row-start-3", Zone { index: 2, card: spell_traps.get(2).copied().flatten(), location: CardLocation::SpellTrapZone, layered } }
            div { class: "col-start-5 row-start-3", Zone { index: 3, card: spell_traps.get(3).copied().flatten(), location: CardLocation::SpellTrapZone, layered } }
            div { class: "col-start-6 row-start-3", Zone { index: 4, card: spell_traps.get(4).copied().flatten(), location: CardLocation::SpellTrapZone, layered } }
            div { class: "col-start-7 row-start-3 relative", PileAnchor { location: CardLocation::Deck } MainDeck {} }
        }
    )
}

#[component]
fn Zone(index: u8, location: CardLocation, card: Option<CardData>, layered: bool) -> Element {
    let state = use_context::<UIState>();
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
                if !layered {
                    InteractiveCard {
                        address: CardAddress { controller: Player::Player, location, sequence: u32::from(index) },
                        card,
                    }
                }

                CardStats { card }
            }

        }
    )
}

#[component]
fn PileAnchor(location: CardLocation) -> Element {
    rsx! {
        LayoutAnchor {
            anchor: Anchor::Pile(location),
            part: AnchorPart::Card,
            class: "absolute left-1/2 top-1/2 h-full aspect-[59/86] -translate-x-1/2 -translate-y-1/2 p-[clamp(1px,0.3vw,5px)] pointer-events-none",
        }
    }
}
