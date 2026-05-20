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
