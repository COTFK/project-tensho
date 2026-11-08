use crate::duel::input::PendingInput;
use dioxus::prelude::*;

use super::constants::ZONE_SIZE;
use super::effect_pile::EffectPile;
use super::extra_deck::ExtraDeck;
use super::layout_anchors::{Anchor, LayoutAnchor, LayoutRevision};
use super::main_deck::MainDeck;
use crate::duel::cards::CardTracker;
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::{CardLocation, Player};

#[component]
pub fn Field() -> Element {
    let revision = use_context::<LayoutRevision>();
    let tracking = use_context::<ReadSignal<Option<CardTracker>>>();
    let pending = use_context::<ReadSignal<Option<PendingInput>>>();
    let effects = use_context::<Signal<Vec<CardData>>>();
    let mut open_pile = use_context::<Signal<Option<CardLocation>>>();
    let has_cards = |location| {
        tracking
            .read()
            .as_ref()
            .and_then(|cards| cards.zone_data(location))
            .is_some_and(|cards| !cards.is_empty())
    };
    let prompted = |location| {
        pending.read().as_ref().is_some_and(|input| {
            input
                .prompted_effects()
                .iter()
                .any(|card| card.location == location)
        })
    };
    let suppressed = !effects.read().is_empty();
    let has_summons = matches!(&*pending.read(), Some(PendingInput::Idle(actions))
        if actions.special_summons.iter().any(|card| card.location == CardLocation::ExtraDeck));
    rsx!(
        div {
            class: "pt-[15vh] max-w-[90vw] w-fit mx-auto grid grid-cols-7 gap-0.5 justify-items-center",
            onresize: move |_| revision.invalidate(),
            div { class: "col-start-3 row-start-1", Zone { index: 5, location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-1", Zone { index: 6, location: CardLocation::MonsterZone } }
            div { class: "col-start-7 row-start-1 relative {ZONE_SIZE}", PileAnchor { location: CardLocation::Banishment } EffectPile {
                location: CardLocation::Banishment,
                has_cards: has_cards(CardLocation::Banishment),
                highlighted: prompted(CardLocation::Banishment) && !suppressed,
                onbrowse: move |location| open_pile.set(Some(location)),
            } }
            div { class: "col-start-1 row-start-2", Zone { index: 5, location: CardLocation::SpellTrapZone } }
            div { class: "col-start-2 row-start-2", Zone { index: 0, location: CardLocation::MonsterZone } }
            div { class: "col-start-3 row-start-2", Zone { index: 1, location: CardLocation::MonsterZone } }
            div { class: "col-start-4 row-start-2", Zone { index: 2, location: CardLocation::MonsterZone } }
            div { class: "col-start-5 row-start-2", Zone { index: 3, location: CardLocation::MonsterZone } }
            div { class: "col-start-6 row-start-2", Zone { index: 4, location: CardLocation::MonsterZone } }
            div { class: "col-start-7 row-start-2 relative", PileAnchor { location: CardLocation::Graveyard } EffectPile {
                location: CardLocation::Graveyard,
                has_cards: has_cards(CardLocation::Graveyard),
                highlighted: prompted(CardLocation::Graveyard) && !suppressed,
                onbrowse: move |location| open_pile.set(Some(location)),
            } }
            div { class: "col-start-1 row-start-3 relative", PileAnchor { location: CardLocation::ExtraDeck } ExtraDeck {
                has_cards: has_cards(CardLocation::ExtraDeck),
                highlighted: has_summons && !suppressed,
                onbrowse: move |_| open_pile.set(Some(CardLocation::ExtraDeck)),
            } }
            div { class: "col-start-2 row-start-3", Zone { index: 0, location: CardLocation::SpellTrapZone } }
            div { class: "col-start-3 row-start-3", Zone { index: 1, location: CardLocation::SpellTrapZone } }
            div { class: "col-start-4 row-start-3", Zone { index: 2, location: CardLocation::SpellTrapZone } }
            div { class: "col-start-5 row-start-3", Zone { index: 3, location: CardLocation::SpellTrapZone } }
            div { class: "col-start-6 row-start-3", Zone { index: 4, location: CardLocation::SpellTrapZone } }
            div { class: "col-start-7 row-start-3 relative", PileAnchor { location: CardLocation::Deck } MainDeck {} }
        }
    )
}

#[component]
fn Zone(index: u8, location: CardLocation) -> Element {
    let pending = use_context::<ReadSignal<Option<PendingInput>>>();
    let onresponse = use_context::<EventHandler<Response>>();
    let placeable_on = matches!(pending(), Some(PendingInput::Place(zones))
        if zones.iter().any(|zone| zone.location == location && zone.sequence == index));
    rsx!(
        div {
            id: "{location}-{index}",
            class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center",
            class: if placeable_on {"border-2 border-yellow-300"} else {"border-0.5"},
            onclick: move |_| {
                if placeable_on {
                    onresponse.call(Response::Place {
                        controller: Player::Player as u8,
                        location: location as u8,
                        index,
                    });
                }
            },
            LayoutAnchor {
                anchor: Anchor::Zone(location, u32::from(index)),
                class: "absolute h-full aspect-[59/86] p-[clamp(1px,0.3vw,5px)] pointer-events-none",
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
                    class: "w-full aspect-[59/86]",
                }
                }
            }
        }
    }
}
