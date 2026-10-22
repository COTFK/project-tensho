mod animations;
mod animator;
mod properties;

pub use animations::*;
pub use animator::Animator;
pub use properties::AnimationController;
pub use properties::AnimationOutcome;
pub use properties::AnimationRequest;
pub use properties::AnimationStatus;
pub use properties::AnimationTrack;

use dioxus::prelude::*;
use web_sys::window;

pub static ANIMATION_CONTROLLER: GlobalSignal<AnimationController> =
    Signal::global(Default::default);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AnimationBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub trait Animation {
    fn keyframes(
        &self,
        name: &str,
        source: AnimationBounds,
        destination: AnimationBounds,
    ) -> String;
    fn duration_ms(&self) -> u32;
    fn parameters(&self, _name: &str) -> String {
        String::new()
    }
}

impl AnimationBounds {
    fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
    }
}

pub fn cancel_animations() {
    let completions = ANIMATION_CONTROLLER.with_mut(AnimationController::clear);
    for completion in completions {
        completion();
    }
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
