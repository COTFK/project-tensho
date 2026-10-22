use dioxus::prelude::*;

use super::super::{Animation, AnimationBounds, scale_between};
use super::AnimationCard;
use crate::ocgcore::CardData;

#[component]
pub fn DrawCard(card: CardData) -> Element {
    rsx! {
        div {
            class: "w-full",
            style: "transform-style: preserve-3d; perspective: 1000px; animation: var(--draw-pose) var(--track-duration) linear var(--track-delay) both;",
            div {
                class: "relative w-full aspect-[59/86]",
                style: "transform-style: preserve-3d; will-change: transform; animation: var(--draw-flip) 150ms ease-in-out calc(var(--track-delay) + 35ms) both;",
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
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DrawToHand {
    rotation: i32,
    translate_y: i32,
    pub delay_ms: u32,
    duration_ms: u32,
}

impl DrawToHand {
    pub fn new(rotation: i32, translate_y: i32, draw_index: usize, opening: bool) -> Self {
        let stagger_ms = if opening { 75 } else { 60 };
        let delay_ms = draw_index as u32 * stagger_ms;
        let duration_ms = if opening { 300 } else { 220 };

        Self {
            rotation,
            translate_y,
            delay_ms,
            duration_ms,
        }
    }
}

impl Animation for DrawToHand {
    fn keyframes(
        &self,
        name: &str,
        source: AnimationBounds,
        destination: AnimationBounds,
    ) -> String {
        let destination_scale = scale_between(source, destination);
        let control_x = source.x + (destination.x - source.x) * 0.6;
        let control_y = source.y + (destination.y - source.y) * 0.12;
        let mut travel = String::new();
        let mut pose = String::new();

        for step in 0..=20 {
            let time = f64::from(step) / 20.0;
            let progress = 1.0 - (1.0 - time).powi(3);
            let inverse = 1.0 - progress;
            let x = inverse * inverse * source.x
                + 2.0 * inverse * progress * control_x
                + progress * progress * destination.x;
            let y = inverse * inverse * source.y
                + 2.0 * inverse * progress * control_y
                + progress * progress * destination.y;
            let scale = 1.0 + (destination_scale - 1.0) * progress;
            let rotation = 5.0 * (std::f64::consts::PI * progress).sin();
            let percentage = time * 100.0;

            travel.push_str(&format!(
                "{percentage:.4}% {{ transform: translate3d({x}px, {y}px, 0) scale({scale}) rotateZ({rotation}deg); }}"
            ));
            pose.push_str(&format!(
                "{percentage:.4}% {{ transform: rotateZ({}deg) translateY({}%); }}",
                f64::from(self.rotation) * progress,
                f64::from(self.translate_y) * progress,
            ));
        }

        format!(
            "@keyframes {name} {{
                {travel}
                100% {{
                    transform: translate3d({}px, {}px, 0) scale({destination_scale});
                }}
            }}
            @keyframes {name}-pose {{
                {pose}
            }}
            @keyframes {name}-flip {{
                0% {{ transform: rotateY(0deg); }}
                100% {{ transform: rotateY(180deg); }}
            }}",
            destination.x, destination.y,
        )
    }

    fn duration_ms(&self) -> u32 {
        self.duration_ms
    }

    fn parameters(&self, name: &str) -> String {
        format!("--draw-pose: {name}-pose; --draw-flip: {name}-flip;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_tracks_overlap_and_land_directly_for_custom_hand_sizes() {
        for count in [1, 3, 5, 7, 10] {
            let tracks: Vec<_> = (0..count)
                .map(|index| DrawToHand::new(0, 0, index, true))
                .collect();
            for (index, track) in tracks.iter().enumerate() {
                assert_eq!(track.delay_ms, index as u32 * 75);
                assert_eq!(track.duration_ms(), 300);
            }
            for pair in tracks.windows(2) {
                assert!(pair[1].delay_ms < pair[0].delay_ms + pair[0].duration_ms());
            }
            let last = tracks.last().unwrap();
            assert_eq!(
                last.delay_ms + last.duration_ms(),
                (count as u32 - 1) * 75 + 300
            );
        }
        let last = DrawToHand::new(0, 0, 4, true);
        assert_eq!(last.delay_ms + last.duration_ms(), 600);
    }

    #[test]
    fn ordinary_draws_keep_their_shorter_flight() {
        let draw = DrawToHand::new(4, 4, 0, false);
        assert_eq!(draw.delay_ms, 0);
        assert_eq!(draw.duration_ms(), 220);
    }

    #[test]
    fn keyframes_are_namespaced_and_end_at_the_real_hand_pose() {
        let draw = DrawToHand::new(-4, 4, 0, true);
        let source = AnimationBounds {
            x: 900.0,
            y: 400.0,
            width: 59.0,
            height: 86.0,
        };
        let destination = AnimationBounds {
            x: 200.0,
            y: 700.0,
            width: 118.0,
            height: 172.0,
        };
        let keyframes = draw.keyframes("test-track", source, destination);
        assert!(keyframes.contains("@keyframes test-track {"));
        assert!(keyframes.contains("@keyframes test-track-pose {"));
        assert!(keyframes.contains("@keyframes test-track-flip {"));
        assert!(keyframes.contains("translate3d(200px, 700px, 0) scale(2)"));
        assert!(keyframes.contains("rotateZ(-4deg) translateY(4%)"));
        assert!(!keyframes.contains("NaN"));
        assert!(!keyframes.contains("inf"));
    }
}
