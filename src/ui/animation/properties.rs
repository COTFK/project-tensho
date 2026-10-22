use std::collections::VecDeque;

use dioxus::prelude::*;

use super::{Animation, AnimationBounds, get_element_bounds};

pub type AnimationCompletion = Box<dyn FnOnce()>;
type FinishHandler = Box<dyn FnOnce(AnimationOutcome)>;

pub struct AnimationTrack {
    pub element: Element,
    pub animation: Box<dyn Animation>,
    source_id: String,
    destination_id: Option<String>,
    pub source_bounds: AnimationBounds,
    pub destination_bounds: AnimationBounds,
    pub delay_ms: u32,
    hide_source: bool,
    pub finished: bool,
}

impl AnimationTrack {
    pub fn new(
        element: Element,
        animation: Box<dyn Animation>,
        source_id: impl Into<String>,
    ) -> Self {
        Self {
            element,
            animation,
            source_id: source_id.into(),
            destination_id: None,
            source_bounds: AnimationBounds::default(),
            destination_bounds: AnimationBounds::default(),
            delay_ms: 0,
            hide_source: true,
            finished: false,
        }
    }

    pub fn between(
        element: Element,
        animation: Box<dyn Animation>,
        source_id: impl Into<String>,
        destination_id: impl Into<String>,
    ) -> Self {
        let mut track = Self::new(element, animation, source_id);
        track.destination_id = Some(destination_id.into());
        track
    }

    pub fn delayed(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    pub fn preserve_source(mut self) -> Self {
        self.hide_source = false;
        self
    }

    pub fn name(request_id: u64, track_id: usize) -> String {
        format!("duel-animation-{request_id}-{track_id}")
    }
}

/// Tracks retain their visual copies until the entire request can hand off together.
pub struct AnimationRequest {
    pub id: u64,
    pub tracks: Vec<AnimationTrack>,
    pub status: AnimationStatus,
    on_finish: Option<FinishHandler>,
}

impl AnimationRequest {
    pub fn new(tracks: Vec<AnimationTrack>) -> Self {
        assert!(
            !tracks.is_empty(),
            "An animation request needs at least one track"
        );
        assert!(
            tracks.iter().all(|track| track.animation.duration_ms() > 0),
            "Animation tracks need a positive duration"
        );
        assert!(
            tracks.len() == 1 || tracks.iter().all(|track| track.destination_id.is_some()),
            "Grouped animation tracks need explicit destinations"
        );
        let status = if tracks.iter().any(|track| track.destination_id.is_none()) {
            AnimationStatus::AwaitingDestination
        } else {
            AnimationStatus::WaitingForLayout
        };

        Self {
            id: 0,
            tracks,
            status,
            on_finish: None,
        }
    }

    pub fn set_on_finish(&mut self, handler: impl FnOnce(AnimationOutcome) + 'static) {
        self.on_finish = Some(Box::new(handler));
    }

    pub fn duration_ms(&self) -> u32 {
        self.tracks
            .iter()
            .map(|track| track.delay_ms.saturating_add(track.animation.duration_ms()))
            .max()
            .expect("Animation requests cannot be empty")
    }

    fn terminate(&mut self, outcome: AnimationOutcome) -> Option<AnimationCompletion> {
        self.status = match outcome {
            AnimationOutcome::Completed => AnimationStatus::Completed,
            AnimationOutcome::Cancelled => AnimationStatus::Cancelled,
            AnimationOutcome::Failed => AnimationStatus::Failed,
        };
        self.on_finish
            .take()
            .map(|handler| Box::new(move || handler(outcome)) as AnimationCompletion)
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
    }

    pub fn try_start_current(&mut self, request_id: u64) -> Result<bool, String> {
        self.try_start_with(request_id, get_element_bounds)
    }

    fn try_start_with(
        &mut self,
        request_id: u64,
        mut measure: impl FnMut(&str) -> Option<AnimationBounds>,
    ) -> Result<bool, String> {
        let Some(current) = self.requests.front_mut() else {
            return Ok(false);
        };
        if current.id != request_id || current.status != AnimationStatus::WaitingForLayout {
            return Ok(false);
        }

        let mut bounds = Vec::with_capacity(current.tracks.len());
        for track in &current.tracks {
            let source = measure(&track.source_id)
                .filter(|bounds| bounds.is_valid())
                .ok_or_else(|| {
                    format!(
                        "Source '{}' is missing or has invalid bounds",
                        track.source_id
                    )
                })?;
            let destination_id = track
                .destination_id
                .as_deref()
                .expect("Waiting-for-layout tracks have a destination");
            let destination = measure(destination_id)
                .filter(|bounds| bounds.is_valid())
                .ok_or_else(|| {
                    format!("Destination '{destination_id}' is missing or has invalid bounds")
                })?;
            bounds.push((source, destination));
        }

        for (track, (source, destination)) in current.tracks.iter_mut().zip(bounds) {
            track.source_bounds = source;
            track.destination_bounds = destination;
        }
        current.status = AnimationStatus::Running;
        Ok(true)
    }

    pub fn set_current_destination(
        &mut self,
        destination_id: impl Into<String>,
        handler: impl FnOnce(AnimationOutcome) + 'static,
    ) -> bool {
        let Some(current) = self.requests.front_mut() else {
            return false;
        };
        if current.status != AnimationStatus::AwaitingDestination || current.tracks.len() != 1 {
            return false;
        }

        current.tracks[0].destination_id = Some(destination_id.into());
        current.set_on_finish(handler);
        current.status = AnimationStatus::WaitingForLayout;
        true
    }

    pub fn finish_track(
        &mut self,
        request_id: u64,
        track_id: usize,
        animation_name: &str,
    ) -> Option<AnimationCompletion> {
        let current = self.requests.front_mut()?;
        if current.id != request_id
            || current.status != AnimationStatus::Running
            || animation_name != AnimationTrack::name(request_id, track_id)
        {
            return None;
        }
        let track = current.tracks.get_mut(track_id)?;
        if track.finished {
            return None;
        }
        track.finished = true;
        if current.tracks.iter().all(|track| track.finished) {
            current.terminate(AnimationOutcome::Completed)
        } else {
            None
        }
    }

    pub fn fail_current(&mut self, request_id: u64, reason: &str) -> Option<AnimationCompletion> {
        let current = self.requests.front_mut()?;
        if current.id != request_id || current.status.is_terminal() {
            return None;
        }
        tracing::error!(request_id, reason, "Animation playback failed");
        current.terminate(AnimationOutcome::Failed)
    }

    pub fn advance(&mut self, request_id: u64) {
        if self
            .current()
            .is_some_and(|request| request.id == request_id && request.status.is_terminal())
        {
            self.requests.pop_front();
        }
    }

    pub fn blocks_game_loop(&self) -> bool {
        self.current()
            .is_some_and(|request| request.status != AnimationStatus::AwaitingDestination)
    }

    pub fn is_source_hidden(&self, element_id: &str) -> bool {
        self.current().is_some_and(|request| {
            request.status == AnimationStatus::Running
                && request
                    .tracks
                    .iter()
                    .any(|track| track.hide_source && track.source_id == element_id)
        })
    }

    pub fn is_destination_pending(&self, element_id: &str) -> bool {
        self.requests.iter().any(|request| {
            !request.status.is_terminal()
                && request
                    .tracks
                    .iter()
                    .any(|track| track.destination_id.as_deref() == Some(element_id))
        })
    }

    pub fn clear(&mut self) -> Vec<AnimationCompletion> {
        self.requests
            .drain(..)
            .filter_map(|mut request| {
                if request.status.is_terminal() {
                    None
                } else {
                    request.terminate(AnimationOutcome::Cancelled)
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationStatus {
    AwaitingDestination,
    WaitingForLayout,
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl AnimationStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationOutcome {
    Completed,
    Cancelled,
    Failed,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::ui::animation::NormalSummon;

    fn bounds() -> AnimationBounds {
        AnimationBounds {
            x: 10.0,
            y: 20.0,
            width: 59.0,
            height: 86.0,
        }
    }

    fn track(destination: &str) -> AnimationTrack {
        AnimationTrack::between(rsx! {}, Box::new(NormalSummon), "source", destination)
    }

    fn start(controller: &mut AnimationController) -> u64 {
        let id = controller.current().unwrap().id;
        assert_eq!(controller.try_start_with(id, |_| Some(bounds())), Ok(true));
        id
    }

    fn finish(controller: &mut AnimationController, id: u64, track_id: usize) {
        if let Some(completion) =
            controller.finish_track(id, track_id, &AnimationTrack::name(id, track_id))
        {
            completion();
        }
    }

    fn record(
        tracks: Vec<AnimationTrack>,
        outcomes: Rc<RefCell<Vec<AnimationOutcome>>>,
    ) -> AnimationRequest {
        let mut request = AnimationRequest::new(tracks);
        request.set_on_finish(move |outcome| outcomes.borrow_mut().push(outcome));
        request
    }

    #[test]
    fn batch_waits_for_every_track_and_finishes_once() {
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let mut controller = AnimationController::default();
        controller.enqueue(record(
            vec![track("first"), track("last").delayed(75)],
            outcomes.clone(),
        ));
        assert_eq!(controller.current().unwrap().duration_ms(), 525);
        let id = start(&mut controller);

        finish(&mut controller, id, 1);
        finish(&mut controller, id, 1);
        assert_eq!(
            controller.current().unwrap().status,
            AnimationStatus::Running
        );
        assert!(outcomes.borrow().is_empty());
        assert!(controller.is_destination_pending("last"));
        assert!(controller.is_destination_pending("first"));

        finish(&mut controller, id, 0);
        finish(&mut controller, id, 0);
        assert_eq!(*outcomes.borrow(), vec![AnimationOutcome::Completed]);
        assert!(controller.blocks_game_loop());
        assert!(!controller.is_destination_pending("first"));
        controller.advance(id);
        assert!(!controller.blocks_game_loop());
    }

    #[test]
    fn child_and_stale_events_cannot_complete_a_track() {
        let mut controller = AnimationController::default();
        controller.enqueue(AnimationRequest::new(vec![track("target")]));
        let id = start(&mut controller);
        controller.finish_track(id, 0, &format!("{}-flip", AnimationTrack::name(id, 0)));
        controller.finish_track(id + 1, 0, &AnimationTrack::name(id, 0));
        controller.finish_track(id, 1, &AnimationTrack::name(id, 1));
        assert!(!controller.current().unwrap().tracks[0].finished);
        controller.advance(id);
        assert_eq!(
            controller.current().unwrap().status,
            AnimationStatus::Running
        );
    }

    #[test]
    fn destinations_hand_off_together_regardless_of_completion_order() {
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut controller = AnimationController::default();
            let destinations = ["first", "second", "third"];
            controller.enqueue(AnimationRequest::new(
                destinations
                    .iter()
                    .map(|destination| track(destination))
                    .collect(),
            ));
            let id = start(&mut controller);

            for (completed, track_id) in order.into_iter().enumerate() {
                finish(&mut controller, id, track_id);
                for destination in destinations {
                    assert_eq!(
                        controller.is_destination_pending(destination),
                        completed < 2,
                        "Destination '{destination}' switched layers before the batch finished"
                    );
                }
            }
        }
    }

    #[test]
    fn completed_tracks_keep_their_sources_hidden_until_batch_handoff() {
        let mut controller = AnimationController::default();
        controller.enqueue(AnimationRequest::new(vec![
            AnimationTrack::between(rsx! {}, Box::new(NormalSummon), "first-source", "first"),
            AnimationTrack::between(rsx! {}, Box::new(NormalSummon), "second-source", "second"),
        ]));
        let id = start(&mut controller);

        finish(&mut controller, id, 0);
        assert!(controller.is_source_hidden("first-source"));
        assert!(controller.is_source_hidden("second-source"));
        finish(&mut controller, id, 1);
        assert!(!controller.is_source_hidden("first-source"));
        assert!(!controller.is_source_hidden("second-source"));
    }

    #[test]
    fn destination_choice_is_not_a_layout_or_game_loop_wait() {
        let mut controller = AnimationController::default();
        controller.enqueue(AnimationRequest::new(vec![AnimationTrack::new(
            rsx! {},
            Box::new(NormalSummon),
            "source",
        )]));
        let id = controller.current().unwrap().id;
        assert!(!controller.blocks_game_loop());
        assert_eq!(
            controller.try_start_with(id, |_| panic!("Must not measure yet")),
            Ok(false)
        );
        assert!(controller.set_current_destination("zone", |_| {}));
        assert!(controller.blocks_game_loop());
        assert!(!controller.set_current_destination("another-zone", |_| {}));
        assert_eq!(start(&mut controller), id);
    }

    #[test]
    fn layout_readiness_is_atomic_and_retries_missing_targets() {
        let mut controller = AnimationController::default();
        controller.enqueue(AnimationRequest::new(vec![
            track("first"),
            track("missing"),
        ]));
        let id = controller.current().unwrap().id;
        assert!(
            controller
                .try_start_with(id, |name| (name != "missing").then(bounds))
                .is_err()
        );
        let request = controller.current().unwrap();
        assert_eq!(request.status, AnimationStatus::WaitingForLayout);
        assert_eq!(request.tracks[0].source_bounds, AnimationBounds::default());
        assert_eq!(start(&mut controller), id);
    }

    #[test]
    fn zero_size_and_non_finite_bounds_are_not_ready() {
        for invalid in [
            AnimationBounds {
                width: 0.0,
                ..bounds()
            },
            AnimationBounds {
                height: -1.0,
                ..bounds()
            },
            AnimationBounds {
                x: f64::NAN,
                ..bounds()
            },
            AnimationBounds {
                y: f64::INFINITY,
                ..bounds()
            },
        ] {
            let mut controller = AnimationController::default();
            controller.enqueue(AnimationRequest::new(vec![track("target")]));
            assert!(controller.try_start_with(0, |_| Some(invalid)).is_err());
            assert_eq!(
                controller.current().unwrap().status,
                AnimationStatus::WaitingForLayout
            );
        }
    }

    #[test]
    fn failed_playback_reveals_destinations_and_releases_after_handoff() {
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let mut controller = AnimationController::default();
        controller.enqueue(record(vec![track("target")], outcomes.clone()));
        let id = controller.current().unwrap().id;
        controller.fail_current(id, "Missing target").unwrap()();
        assert!(!controller.is_destination_pending("target"));
        assert!(!controller.is_source_hidden("source"));
        assert!(controller.fail_current(id, "Duplicate failure").is_none());
        assert_eq!(*outcomes.borrow(), vec![AnimationOutcome::Failed]);
        controller.advance(id);
        assert!(!controller.blocks_game_loop());
    }

    #[test]
    fn cancellation_covers_running_and_queued_requests_without_reusing_ids() {
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let mut controller = AnimationController::default();
        controller.enqueue(record(vec![track("first")], outcomes.clone()));
        controller.enqueue(record(vec![track("second")], outcomes.clone()));
        let old_id = start(&mut controller);
        for completion in controller.clear() {
            completion();
        }
        assert_eq!(*outcomes.borrow(), vec![AnimationOutcome::Cancelled; 2]);
        assert!(!controller.blocks_game_loop());
        assert!(!controller.is_destination_pending("second"));
        assert!(controller.clear().is_empty());

        controller.enqueue(AnimationRequest::new(vec![track("new")]));
        let new_id = start(&mut controller);
        assert!(new_id > old_id);
        finish(&mut controller, old_id, 0);
        controller.fail_current(old_id, "Old watchdog");
        controller.advance(old_id);
        assert_eq!(controller.current().unwrap().id, new_id);
        assert_eq!(
            controller.current().unwrap().status,
            AnimationStatus::Running
        );
    }

    #[test]
    fn sources_can_be_preserved_and_tracks_do_not_start_the_next_action() {
        let mut controller = AnimationController::default();
        controller.enqueue(AnimationRequest::new(vec![
            track("first").preserve_source(),
        ]));
        controller.enqueue(AnimationRequest::new(vec![track("second")]));
        let first_id = start(&mut controller);
        assert!(!controller.is_source_hidden("source"));
        finish(&mut controller, first_id, 0);
        controller.advance(first_id);
        assert_eq!(
            controller.current().unwrap().status,
            AnimationStatus::WaitingForLayout
        );
        start(&mut controller);
        assert!(controller.is_source_hidden("source"));
    }

    #[test]
    fn terminal_requests_are_not_cancelled_again() {
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let mut controller = AnimationController::default();
        controller.enqueue(record(vec![track("target")], outcomes.clone()));
        let id = start(&mut controller);
        finish(&mut controller, id, 0);
        assert!(controller.clear().is_empty());
        assert_eq!(*outcomes.borrow(), vec![AnimationOutcome::Completed]);
    }
}
