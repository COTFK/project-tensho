use dioxus::prelude::*;

use super::constants::ZONE_SIZE;
use crate::state::UIState;
use crate::ui::components::CardStack;
use crate::utility::CARD_BACK;

#[component]
pub fn MainDeck() -> Element {
    let state = use_context::<UIState>();

    rsx!(
        div {
            class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5",
            div {
                class: "absolute inset-[clamp(2px,0.6vw,8px)] flex justify-center invisible pointer-events-none",
                div {
                    id: "main-deck-animation-source",
                    class: "h-full aspect-[59/86]",
                    transform: "translate(0.01vw, -0.01vh)",
                }
            }
            CardStack {
                length: (state.main_deck_length)() as usize,
                image_url: CARD_BACK,
            }
        }
    )
}
