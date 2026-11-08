use dioxus::prelude::*;

use super::layout_anchors::{LayoutAnchor, LayoutAnchorId};

#[component]
pub fn HandLayout(hand_size: usize) -> Element {
    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[27.5%] z-50 pointer-events-none",
            class: if hand_size == 0 { "min-h-[14.576271vw]" },
            LayoutAnchor {
                anchor: LayoutAnchorId::HandBase,
                class: "absolute left-1/2 top-0 w-[10vw] aspect-[59/86] -translate-x-1/2 pointer-events-none",
            }
            for index in 0..hand_size {
                div {
                    key: "{index}",
                    class: "w-[10vw] aspect-[59/86] shrink-0 -mx-[1.1vw] relative",
                }
            }
        }
    )
}
