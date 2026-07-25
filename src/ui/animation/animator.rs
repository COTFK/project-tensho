use dioxus::prelude::*;

use crate::ui::animation::{AnimationStatus, CURRENT_ANIMATION};

#[component]
pub fn Animator() -> Element {
    let properties = CURRENT_ANIMATION.read();
    let Some(properties) = properties.as_ref() else {
        return rsx! {};
    };

    rsx!(
        style { {properties.animation.keyframes(properties.source, properties.destination)} }
        if properties.status == AnimationStatus::Running {
            div {
                class: "fixed inset-0 overflow-hidden pointer-events-none",
                z_index: "100",
                div {
                    class: "absolute",
                    style: properties.animation.parameters(),
                    onanimationend: move |_| {
                        CURRENT_ANIMATION.with_mut(|animation| {
                            if let Some(properties) = animation.as_mut() {
                                properties.status = AnimationStatus::Done;
                            }
                        });
                    },
                    div {
                        style: "width: {properties.source.width}px;",
                        {properties.element.clone()}
                    }
                }
            }
        }
    )
}
