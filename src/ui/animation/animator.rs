use std::time::Duration;

use dioxus::prelude::*;
use futures_timer::Delay;

use super::{ANIMATION_CONTROLLER, AnimationStatus, AnimationTrack, cancel_animations};

#[component]
pub fn Animator() -> Element {
    use_drop(cancel_animations);
    let controller = ANIMATION_CONTROLLER.read();

    rsx! {
        if let Some(request) = controller.current() {
            Playback { key: "{request.id}", request_id: request.id }
        }
    }
}

#[component]
fn Playback(request_id: u64) -> Element {
    let phase = use_memo(move || {
        ANIMATION_CONTROLLER
            .read()
            .current()
            .filter(|request| request.id == request_id)
            .map(|request| (request.status, request.duration_ms()))
    });

    use_effect(move || {
        let Some((status, duration_ms)) = phase() else {
            return;
        };
        match status {
            AnimationStatus::WaitingForLayout => {
                spawn(async move {
                    for attempt in 0..=60 {
                        let result = ANIMATION_CONTROLLER
                            .with_mut(|controller| controller.try_start_current(request_id));
                        match result {
                            Ok(_) => return,
                            Err(reason) if attempt == 60 => {
                                let completion = ANIMATION_CONTROLLER.with_mut(|controller| {
                                    controller.fail_current(request_id, &reason)
                                });
                                if let Some(completion) = completion {
                                    completion();
                                }
                            }
                            Err(_) => Delay::new(Duration::from_millis(16)).await,
                        }
                    }
                });
            }
            AnimationStatus::Running => {
                spawn(async move {
                    Delay::new(Duration::from_millis(u64::from(duration_ms) + 1500)).await;
                    let completion = ANIMATION_CONTROLLER.with_mut(|controller| {
                        controller.fail_current(request_id, "Timed out waiting for animationend")
                    });
                    if let Some(completion) = completion {
                        completion();
                    }
                });
            }
            _ => {}
        }
    });

    let controller = ANIMATION_CONTROLLER.read();
    let Some(request) = controller
        .current()
        .filter(|request| request.id == request_id)
    else {
        return rsx! {};
    };

    rsx! {
        if request.status == AnimationStatus::Running {
            div {
                class: "fixed inset-0 overflow-hidden pointer-events-none z-[100]",
                for (track_id, track) in request.tracks.iter().enumerate() {
                    {
                            let name = AnimationTrack::name(request_id, track_id);
                            let duration_ms = track.animation.duration_ms();
                            let delay_ms = track.delay_ms;
                            let parameters = track.animation.parameters(&name);
                            rsx! {
                                div {
                                    key: "{request_id}-{track_id}",
                                    class: "absolute motion-reduce:![animation-duration:1ms] motion-reduce:![animation-delay:0ms]",
                                    z_index: "{track_id}",
                                    style: "--track-duration: {duration_ms}ms; --track-delay: {delay_ms}ms; transform-origin: top left; will-change: transform; {parameters} animation: {name} {duration_ms}ms linear {delay_ms}ms both;",
                                    onanimationend: move |event: AnimationEvent| {
                                        let completion = ANIMATION_CONTROLLER.with_mut(|controller| {
                                            controller.finish_track(
                                                request_id,
                                                track_id,
                                                &event.data().animation_name(),
                                            )
                                        });
                                        if let Some(completion) = completion {
                                            completion();
                                        }
                                    },
                                    style {
                                        {track.animation.keyframes(&name, track.source_bounds, track.destination_bounds)}
                                    }
                                    div {
                                        style: "width: {track.source_bounds.width}px;",
                                        {track.element.clone()}
                                    }
                            }
                        }
                    }
                }
            }
        }
        if request.status.is_terminal() {
            div {
                key: "finished-{request_id}",
                class: "hidden",
                onmounted: move |_| {
                    ANIMATION_CONTROLLER.with_mut(|controller| controller.advance(request_id));
                },
            }
        }
    }
}
