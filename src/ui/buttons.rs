use dioxus::prelude::*;
use web_sys::window;

#[component]
pub fn FullscreenButton() -> Element {
    let toggle_fullscreen = move |_| {
        if let Some(document) = window().and_then(|window| window.document()) {
            if document.fullscreen_element().is_some() {
                let _ = document.exit_fullscreen();
            } else if let Some(element) = document.document_element() {
                let _ = element.request_fullscreen();
            }
        }
    };

    rsx!(
        button {
            class: "fixed top-3 right-3 z-50 w-8 h-8 rounded-md border border-white/20 bg-black/70 text-[10px] font-semibold text-white shadow-lg backdrop-blur-sm flex items-center justify-center cursor-pointer hover:bg-black/85",
            aria_label: "Toggle fullscreen",
            onclick: toggle_fullscreen,
            svg {
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                class: "w-5 h-5",
                fill: "currentColor",
                stroke: "currentColor",
                stroke_width: "0.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M3 3h6v2H5v4H3V3zm18 0v6h-2V5h-4V3h6zM3 21v-6h2v4h4v2H3zm18-6v6h-6v-2h4v-4h2z" }
            }
        }
    )
}

#[component]
pub fn ResetButton(onclick: EventHandler<MouseEvent>) -> Element {
    rsx!(
        button {
            class: "fixed top-3 left-3 z-50 w-8 h-8 rounded-md border border-white/20 bg-black/70 text-[10px] font-semibold text-white shadow-lg backdrop-blur-sm flex items-center justify-center cursor-pointer hover:bg-black/85",
            aria_label: "Restart",
            onclick: onclick,
            svg {
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                class: "w-5 h-5",
                fill: "none",
                stroke: "currentColor",
                color: "currentColor",
                stroke_width: "2",
                stroke_linecap: "square",
                stroke_linejoin: "miter",
                polyline { points: "22 12 19 15 16 12" }
                path { d: "M11,20 C6.581722,20 3,16.418278 3,12 C3,7.581722 6.581722,4 11,4 C15.418278,4 19,7.581722 19,12 L19,14" }
            }
        }
    )
}
