use dioxus::prelude::*;

use super::super::{Animation, AnimationBounds, scale_between};
use super::AnimationCard;
use crate::ocgcore::CardData;

#[component]
pub fn DrawCard(card: CardData, rotation: i32, translate_y: i32) -> Element {
    rsx!(div {
        class: "w-full",
        style: "transform: translateZ(0) rotateZ({rotation}deg) translateY({translate_y}%); transform-style: preserve-3d; backface-visibility: hidden;",
        perspective: "1000px",
        div {
            class: "relative w-full aspect-[59/86]",
            style: "transform: translateZ(0); transform-style: preserve-3d; backface-visibility: hidden; will-change: transform; animation: draw-card-flip var(--draw-duration) cubic-bezier(0.4, 0, 0.2, 1) forwards;",
            div {
                class: "absolute inset-0",
                style: "backface-visibility: hidden; -webkit-backface-visibility: hidden;",
                AnimationCard { card, facedown: true }
            }
            div {
                class: "absolute inset-0",
                style: "backface-visibility: hidden; -webkit-backface-visibility: hidden; transform: rotateY(180deg);",
                AnimationCard { card, facedown: false }
            }
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrawToHand;

impl Animation for DrawToHand {
    fn name(&self) -> &'static str {
        "draw-to-hand"
    }

    fn keyframes(&self, source: AnimationBounds, destination: AnimationBounds) -> String {
        let destination_scale = scale_between(source, destination);
        let control_x = source.x + (destination.x - source.x) * 0.6;
        let control_y = source.y + (destination.y - source.y) * 0.12;
        let mut travel_keyframes = String::new();

        for step in 0..=10 {
            let progress = step as f64 / 10.0;
            let inverse = 1.0 - progress;
            let x = inverse * inverse * source.x
                + 2.0 * inverse * progress * control_x
                + progress * progress * destination.x;
            let y = inverse * inverse * source.y
                + 2.0 * inverse * progress * control_y
                + progress * progress * destination.y;
            let scale = 1.0 + (destination_scale - 1.0) * progress;
            let rotation = 1.5 * inverse;

            travel_keyframes.push_str(&format!(
                "{}% {{ transform: translate3d({x}px, {y}px, 0) scale({scale}) rotateZ({rotation}deg); }}",
                step * 10,
            ));
        }

        format!(
            "@keyframes draw-to-hand {{
                {travel_keyframes}
            }}
            @keyframes draw-card-flip {{
                0% {{ transform: translateZ(0) rotateY(0deg); }}
                80%, 100% {{ transform: translateZ(0) rotateY(180deg); }}
            }}"
        )
    }

    fn parameters(&self) -> &'static str {
        "--draw-duration: 175ms; transform-origin: top left; backface-visibility: hidden; contain: layout paint; will-change: transform; animation: draw-to-hand var(--draw-duration) linear forwards;"
    }
}
