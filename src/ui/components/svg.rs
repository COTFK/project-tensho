use dioxus::prelude::*;

#[component]
pub fn SummonIcon() -> Element {
    rsx!(
        svg {
            view_box: "0 0 24 24",
            xmlns: "http://www.w3.org/2000/svg",
            fill: "currentColor",
            g {
                path {
                    d: "M8,7.29a6.22,6.22,0,0,0-4,5.83L5.12,22H16V18h4V2H8ZM14,20H6.88L6,13.06a4.25,4.25,0,0,1,.91-2.58A4.2,4.2,0,0,1,8,9.5v5.92L11.45,12a1.17,1.17,0,0,1,1.89,1.3L11.24,18H14ZM10,4h8V16H14.32l.85-1.92A3.17,3.17,0,0,0,10,10.55l0,0V4Z",
                }
            }
        }
    )
}

#[component]
pub fn ResetIcon() -> Element {
    rsx!(
        svg {
            view_box: "0 0 24 24",
            xmlns: "http://www.w3.org/2000/svg",
            class: "w-6 h-6",
            fill: "none",
            stroke: "currentColor",
            color: "currentColor",
            stroke_width: "2",
            stroke_linecap: "square",
            stroke_linejoin: "miter",
            polyline { points: "22 12 19 15 16 12" }
            path { d: "M11,20 C6.581722,20 3,16.418278 3,12 C3,7.581722 6.581722,4 11,4 C15.418278,4 19,7.581722 19,12 L19,14" }
        }
    )
}

#[component]
pub fn FullscreenIcon() -> Element {
    rsx!(
        svg {
            view_box: "0 0 24 24",
            xmlns: "http://www.w3.org/2000/svg",
            class: "w-6 h-6",
            fill: "currentColor",
            stroke: "currentColor",
            stroke_width: "0.5",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M3 3h6v2H5v4H3V3zm18 0v6h-2V5h-4V3h6zM3 21v-6h2v4h4v2H3zm18-6v6h-6v-2h4v-4h2z" }
        }
    )
}

#[component]
pub fn Spinner() -> Element {
    rsx!(
        svg {
            class: "size-12 animate-spin text-white",
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            view_box:"0 0 24 24",
            circle {
                class: "opacity-25",
                cx: "12",
                cy: "12",
                r: "10",
                stroke: "currentColor",
                stroke_width: "4",
            }
            path {
                class: "opacity-75",
                fill: "currentColor",
                d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            }
        }
    )
}
