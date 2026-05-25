use dioxus::prelude::*;

#[component]
pub fn UIButton(label: String, class: String, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx!(
        button { 
            class: "{class} w-8 h-8 rounded-md border border-white/20 bg-black/70 text-[10px] font-semibold text-white shadow-lg backdrop-blur-sm flex items-center justify-center cursor-pointer hover:bg-black/85",
            aria_label: label,
            onclick: onclick,
            {children}
        }
    )
}

#[component]
pub fn OptionButton(
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
pub fn ActionButton(label: String, class: String, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx!(
        div {
            class: "flex flex-col items-center justify-center gap-2",
            p {
                class: "text-white text-sm font-semibold shadow-md text-center",
                "{label}"
            },
            button {
                class: "bg-black size-12 p-1 rounded-full border-3 {class} cursor-pointer text-center",
                onclick: onclick,
                {children}
            }
        }
    )
}