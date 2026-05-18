use dioxus::prelude::*;

use super::card::Card;

#[component]
pub fn Field(monsters: WriteSignal<Vec<u32>>) -> Element {
    rsx!(
        div { // Entire field
            class: "mx-auto flex flex-col gap-3 w-min pt-8",
            div { // Extra Monster Zones
                class: "flex flex-row gap-3 justify-evenly",
                div {
                    class: "",
                    Zone {}
                }
                div {
                    class: "",
                    Zone {}
                }
            }
            div { // Main Monster Zones
                class: "flex flex-row gap-3 justify-center",
                Zone { id: monsters().get(0).copied()}
                Zone { id: monsters().get(1).copied()}
                Zone { id: monsters().get(2).copied()}
                Zone { id: monsters().get(3).copied()}
                Zone { id: monsters().get(4).copied()}
            }
            div { // Spell/Trap Zones
                class: "flex flex-row gap-3 justify-center",
                Zone {}
                Zone {}
                Zone {}
                Zone {}
                Zone {}
            }
        }
    )
}

#[component]
fn Zone(id: Option<u32>) -> Element {
    rsx!(
        div {
            class: "border-0.5 shadow-xl bg-slate-50/2 size-[14vw] aspect-square flex items-center justify-center",
            if id != Some(0) && id.is_some() {
                div {
                    class: "w-[9vw]",
                    Card {
                        id: id.unwrap()
                    }
                }
            }
        }
    )
}
