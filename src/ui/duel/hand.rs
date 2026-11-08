use dioxus::prelude::*;

use super::layout_anchors::{Anchor, AnchorPart, LayoutAnchor};

pub(super) fn hand_transform(index: usize, hand_size: usize) -> (i32, i32) {
    let center = hand_size as i32 - 1;
    let distance = index as i32 * 2 - center;
    let translate_y = (distance.abs() * distance.abs() * 4) / 16;
    (distance, translate_y)
}

#[component]
pub fn Hand(hand_size: usize) -> Element {
    rsx!(
        div {
            class: "fixed flex flex-row justify-center self-end place-self-center inset-0 translate-y-[27.5%] z-50 pointer-events-none",
            class: if hand_size == 0 { "min-h-[14.576271vw]" },
            LayoutAnchor {
                anchor: Anchor::HandBase,
                part: AnchorPart::Card,
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
