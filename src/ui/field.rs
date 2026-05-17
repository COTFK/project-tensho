use dioxus::prelude::*;

#[component]
pub fn Field() -> Element {
    rsx!(
        div { // Entire field
            class: "mx-auto flex flex-col gap-3 w-min",
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
                Zone {}
                Zone {}
                Zone {}
                Zone {}
                Zone {}
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
fn Zone() -> Element {
    rsx!(div {
        class: "border-0.5 shadow-md bg-slate-50/20",
        width: "8vw",
        aspect_ratio: "1/1",
    })
}
