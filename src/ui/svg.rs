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
