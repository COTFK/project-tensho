use dioxus::prelude::*;

use super::components::CardStack;
use crate::state::DuelState;
use crate::utility::CARD_BACK;
use crate::utility::EXTRA_BACK;

#[component]
pub fn MainDeck() -> Element {
    let state = use_context::<DuelState>();

    rsx!(
        div {
            class: "relative bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            CardStack {
                length: (state.main_deck_length)(),
                image_url: CARD_BACK,
            }
        }
    )
}

#[component]
pub fn ExtraDeck() -> Element {
    let state = use_context::<DuelState>();

    rsx!(
        div {
            class: "relative bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            CardStack {
                length: (state.extra_deck_length)(),
                image_url: EXTRA_BACK,
            }
        }
    )
}
