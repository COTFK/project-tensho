use dioxus::prelude::*;

use super::constants::ZONE_SIZE;

#[component]
pub fn MainDeck() -> Element {
    rsx!(div {
        class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5",
    })
}
