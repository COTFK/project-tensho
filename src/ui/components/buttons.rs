use crate::ui::svg::SummonIcon;
use dioxus::prelude::*;

#[component]
pub fn BlockButton(
    label: String,
    disabled: Option<bool>,
    onclick: EventHandler<MouseEvent>,
    #[props(default)] additional_classes: String,
) -> Element {
    rsx!(
        button {
            class: "w-fit px-8 py-1 h-min rounded-lg font-semibold text-white cursor-pointer {additional_classes}",
            disabled: disabled == Some(true),
            onclick: onclick,
            "{label}"
        }
    )
}

#[component]
pub fn ActivateButton(onclick: EventHandler<MouseEvent>) -> Element {
    rsx!(
        div {
            class: "flex flex-col items-center justify-center gap-2",
            p {
                class: "text-white text-sm font-semibold shadow-md text-center",
                "Activate"
            },
            button {
                class: "bg-black size-12 p-1 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer text-center",
                onclick: onclick,
                SummonIcon {  }
            }
        }
    )
}

#[component]
pub fn SummonButton(onclick: EventHandler<MouseEvent>) -> Element {
    rsx!(
        div {
            class: "flex flex-col items-center justify-center gap-2",
            p {
                class: "text-white text-sm font-semibold shadow-md text-center",
                "Summon"
            }
            button {
                class: "bg-black size-12 p-1 rounded-full border-3 border-cyan-500 text-cyan-300 cursor-pointer text-center",
                onclick: onclick,
                SummonIcon {}
            }
        }
    )
}
