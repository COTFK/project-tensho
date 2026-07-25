use dioxus::prelude::*;

use crate::ui::animation::{ANIMATION_CONTROLLER, AnimationStatus};

#[component]
pub fn Animator() -> Element {
    let controller = ANIMATION_CONTROLLER.read();
    let Some(properties) = controller.current() else {
        return rsx! {};
    };
    let animation_name = properties.animation.name();

    rsx!(
        style { {properties.animation.keyframes(properties.source_bounds, properties.destination_bounds)} }
        if properties.status == AnimationStatus::Running {
            div {
                class: "fixed inset-0 overflow-hidden pointer-events-none",
                z_index: "100",
                div {
                    key: "{properties.id}",
                    class: "absolute",
                    style: properties.animation.parameters(),
                    onanimationend: move |event: AnimationEvent| {
                        if event.data().animation_name() != animation_name {
                            return;
                        }
                        let completion = ANIMATION_CONTROLLER.with_mut(|controller| {
                            controller.finish_current()
                        });
                        if let Some(completion) = completion {
                            completion();
                        }
                    },
                    div {
                        style: "width: {properties.source_bounds.width}px;",
                        {properties.element.clone()}
                    }
                }
            }
        }
        if properties.status == AnimationStatus::Done {
            div {
                key: "completed-{properties.id}",
                class: "hidden",
                onmounted: move |_| {
                    ANIMATION_CONTROLLER.with_mut(|controller| {
                        controller.advance();
                    });
                },
            }
        }
    )
}
