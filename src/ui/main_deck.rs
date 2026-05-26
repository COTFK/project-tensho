use dioxus::prelude::*;

use super::components::CardStack;
use crate::state::DuelState;
use crate::utility::CARD_BACK;

#[component]
pub fn MainDeck() -> Element {
    let state = use_context::<DuelState>();

    rsx!(
        div {
            class: "relative bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5",
            CardStack {
                length: (state.main_deck_length)() as usize,
                image_url: CARD_BACK,
            }
        }
    )
}
