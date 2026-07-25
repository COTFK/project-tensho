use dioxus::prelude::*;

use super::{Animation, AnimationBounds};

pub struct AnimationRequest {
    pub element: Element,
    pub animation: Box<dyn Animation>,
    pub source: AnimationBounds,
    pub destination: AnimationBounds,
    pub status: AnimationStatus,
}

impl AnimationRequest {
    pub fn new(element: Element, animation: Box<dyn Animation>, source: AnimationBounds) -> Self {
        Self {
            element,
            animation,
            source,
            destination: AnimationBounds::default(),
            status: AnimationStatus::Queued,
        }
    }

    pub fn start(&mut self, destination: AnimationBounds) {
        self.destination = destination;
        self.status = AnimationStatus::Running;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnimationStatus {
    Queued,
    Running,
    Done,
}
