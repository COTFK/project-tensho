use std::collections::VecDeque;

use dioxus::prelude::*;

use super::{Animation, AnimationBounds, get_element_bounds};

pub type AnimationCompletion = Box<dyn FnOnce()>;

pub struct AnimationRequest {
    pub id: u64,
    pub element: Element,
    pub animation: Box<dyn Animation>,
    source_id: String,
    destination_id: Option<String>,
    pub source_bounds: AnimationBounds,
    pub destination_bounds: AnimationBounds,
    pub status: AnimationStatus,
    completion: Option<AnimationCompletion>,
}

impl AnimationRequest {
    pub fn new(
        element: Element,
        animation: Box<dyn Animation>,
        source_id: impl Into<String>,
    ) -> Self {
        Self {
            id: 0,
            element,
            animation,
            source_id: source_id.into(),
            destination_id: None,
            source_bounds: AnimationBounds::default(),
            destination_bounds: AnimationBounds::default(),
            status: AnimationStatus::Queued,
            completion: None,
        }
    }

    pub fn between(
        element: Element,
        animation: Box<dyn Animation>,
        source_id: impl Into<String>,
        destination_id: impl Into<String>,
    ) -> Self {
        let mut request = Self::new(element, animation, source_id);
        request.destination_id = Some(destination_id.into());
        request
    }

    pub fn on_complete(mut self, completion: impl FnOnce() + 'static) -> Self {
        self.completion = Some(Box::new(completion));
        self
    }

    fn set_destination(
        &mut self,
        destination_id: impl Into<String>,
        completion: AnimationCompletion,
    ) {
        self.destination_id = Some(destination_id.into());
        self.completion = Some(completion);
    }

    fn try_start(&mut self) -> bool {
        if self.status != AnimationStatus::Queued {
            return false;
        }
        let Some(source) = get_element_bounds(&self.source_id) else {
            return false;
        };
        let Some(destination) = self.destination_id.as_deref().and_then(get_element_bounds) else {
            return false;
        };

        self.source_bounds = source;
        self.destination_bounds = destination;
        self.status = AnimationStatus::Running;
        true
    }
}

#[derive(Default)]
pub struct AnimationController {
    requests: VecDeque<AnimationRequest>,
    next_id: u64,
}

impl AnimationController {
    pub fn current(&self) -> Option<&AnimationRequest> {
        self.requests.front()
    }

    pub fn enqueue(&mut self, mut request: AnimationRequest) {
        request.id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.requests.push_back(request);
        if self.requests.len() == 1 {
            self.try_start_current();
        }
    }

    pub fn enqueue_all(&mut self, requests: impl IntoIterator<Item = AnimationRequest>) {
        for request in requests {
            self.enqueue(request);
        }
    }

    pub fn try_start_current(&mut self) -> bool {
        self.requests
            .front_mut()
            .is_some_and(AnimationRequest::try_start)
    }

    pub fn set_current_destination(
        &mut self,
        destination_id: impl Into<String>,
        completion: impl FnOnce() + 'static,
    ) -> bool {
        let Some(current) = self.requests.front_mut() else {
            return false;
        };

        current.set_destination(destination_id, Box::new(completion));
        current.try_start()
    }

    pub fn finish_current(&mut self) -> Option<AnimationCompletion> {
        let current = self.requests.front_mut()?;
        if current.status != AnimationStatus::Running {
            return None;
        }

        current.status = AnimationStatus::Done;
        current.completion.take()
    }

    pub fn advance(&mut self) -> bool {
        if !self
            .current()
            .is_some_and(|request| request.status == AnimationStatus::Done)
        {
            return false;
        }

        self.requests.pop_front();
        self.try_start_current();
        true
    }

    pub fn is_running(&self) -> bool {
        self.current()
            .is_some_and(|request| request.status == AnimationStatus::Running)
    }

    pub fn is_running_from(&self, element_id: &str) -> bool {
        self.current().is_some_and(|request| {
            request.status == AnimationStatus::Running && request.source_id == element_id
        })
    }

    pub fn is_destination_pending(&self, element_id: &str) -> bool {
        self.requests.iter().any(|request| {
            request.status != AnimationStatus::Done
                && request.destination_id.as_deref() == Some(element_id)
        })
    }

    pub fn clear(&mut self) {
        self.requests.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnimationStatus {
    Queued,
    Running,
    Done,
}
