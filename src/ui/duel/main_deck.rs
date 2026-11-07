use dioxus::prelude::*;

use super::constants::ZONE_SIZE;
use crate::state::UIState;
use crate::ui::components::CardStack;
use crate::utility::CARD_BACK;

#[component]
pub fn MainDeck(#[props(default)] layered: bool) -> Element {
    let state = use_context::<UIState>();

    rsx!(
        div {
            class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5",
            if !layered {
                CardStack {
                    length: (state.main_deck_length)() as usize,
                    image_url: CARD_BACK,
                }
            }
        }
    )
}
