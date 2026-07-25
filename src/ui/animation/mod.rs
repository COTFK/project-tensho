mod animations;
mod animator;
mod properties;

pub use animations::*;
pub use animator::Animator;
pub use properties::AnimationController;
pub use properties::AnimationRequest;
pub use properties::AnimationStatus;

use dioxus::prelude::*;
use web_sys::window;

pub static ANIMATION_CONTROLLER: GlobalSignal<AnimationController> =
    Signal::global(Default::default);

#[derive(Debug, Clone, Copy, Default)]
pub struct AnimationBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub trait Animation {
    fn name(&self) -> &'static str;
    fn keyframes(&self, source: AnimationBounds, destination: AnimationBounds) -> String;
    fn parameters(&self) -> &'static str;
}

pub fn scale_between(source: AnimationBounds, destination: AnimationBounds) -> f64 {
    if source.width > 0.0 {
        destination.width / source.width
    } else {
        1.0
    }
}

pub fn get_element_bounds(element_id: &str) -> Option<AnimationBounds> {
    let rect = window()?
        .document()?
        .get_element_by_id(element_id)?
        .get_bounding_client_rect();

    Some(AnimationBounds {
        x: rect.left(),
        y: rect.top(),
        width: rect.width(),
        height: rect.height(),
    })
}
