use dioxus::prelude::*;
use crate::ui::svg::SummonIcon;

#[component]
pub fn BlockButton(
    label: String,
    disabled: Option<bool>,
    onclick: EventHandler<MouseEvent>,
    #[props(default)]
    additional_classes: String,
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
        p {
            class: "text-white text-sm font-semibold shadow-md text-center",
            "Activate"
        },
        button {
            class: "bg-black size-12 p-1 rounded-full border-3 border-yellow-500 text-yellow-300 cursor-pointer text-center",
            onclick: onclick,
            SummonIcon {  }
        }
    )
}