use dioxus::prelude::*;

use super::banishment::Banishment;
use super::card_interaction::InteractiveCard;
use super::card_stats::CardStats;
use super::constants::ZONE_SIZE;
use super::extra_deck::ExtraDeck;
use super::graveyard::Graveyard;
use super::main_deck::MainDeck;
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::state::{UIState, send_response};
use crate::ui::card_layer::CardAddress;

#[component]
pub fn Field() -> Element {
    let state = use_context::<UIState>();
    let monsters = (state.monsters)();
    let spell_traps = (state.spell_traps)();
    rsx!(
        div {
            class: "pt-[15vh] max-w-[90vw] w-fit mx-auto grid grid-cols-7 gap-0.5 justify-items-center",
            div { class: "col-start-3 row-start-1", Zone { index: 5, card: monsters.get(5).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-1", Zone { index: 6, card: monsters.get(6).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-7 row-start-1", Banishment {} }
            div { class: "col-start-1 row-start-2", Zone { index: 5, card: spell_traps.get(5).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-2 row-start-2", Zone { index: 0, card: monsters.first().copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-3 row-start-2", Zone { index: 1, card: monsters.get(1).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-4 row-start-2", Zone { index: 2, card: monsters.get(2).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-2", Zone { index: 3, card: monsters.get(3).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-6 row-start-2", Zone { index: 4, card: monsters.get(4).copied().flatten(), location: CardLocation::MonsterZone } }
            div { class: "col-start-7 row-start-2", Graveyard {} }
            div { class: "col-start-1 row-start-3", ExtraDeck {} }
            div { class: "col-start-2 row-start-3", Zone { index: 0, card: spell_traps.first().copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-3 row-start-3", Zone { index: 1, card: spell_traps.get(1).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-4 row-start-3", Zone { index: 2, card: spell_traps.get(2).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-5 row-start-3", Zone { index: 3, card: spell_traps.get(3).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-6 row-start-3", Zone { index: 4, card: spell_traps.get(4).copied().flatten(), location: CardLocation::SpellTrapZone } }
            div { class: "col-start-7 row-start-3", MainDeck {} }
        }
    )
}

#[component]
fn Zone(index: u8, location: CardLocation, card: Option<CardData>) -> Element {
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
            if let Some(card) = card {
                InteractiveCard {
                    address: CardAddress { controller: Player::Player, location, sequence: u32::from(index) },
                    card,
                }
                CardStats { card }
            }
        }
    )
}
