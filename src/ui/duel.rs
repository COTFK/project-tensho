use dioxus::prelude::*;
use web_sys::window;

use super::components::UIButton;
use super::field::Field;
use super::hand::Hand;
use super::modal::ModalContainer;
use crate::ocgcore::Duel;
use crate::state::DuelState;
use crate::state::handle_left_click;
use crate::state::handle_right_click;
use crate::state::run_game_loop;
use crate::ui::components::svg::FullscreenIcon;
use crate::ui::components::svg::ResetIcon;

#[component]
pub fn DuelScreen(duel: Duel, resource_handle: Resource<anyhow::Result<Duel>>) -> Element {
    // Initialize duel state
    let mut state = DuelState::new(duel.clone());
    use_context_provider(move || state);
    use_effect(use_reactive((&duel,), move |(duel,)| {
        state.reset(duel.clone());
    }));
    use_drop(move || (state.duel)().destroy());

    // Start game loop
    use_effect(run_game_loop);

    rsx!(
        main {
            class: "relative h-dvh w-dvw bg-gray-800",
            oncontextmenu: handle_right_click,
            onclick: |_| handle_left_click(),
            UIButton {
                class: "fixed top-3 left-3 z-50",
                label: "Restart game",
                onclick: move |_| {
                    resource_handle.clear();
                    resource_handle.restart();
                },
                ResetIcon {}
            }
            UIButton {
                label: "Toggle fullscreen",
                class: "fixed top-3 right-3 z-50",
                onclick: move |_| {
                    if let Some(document) = window().and_then(|window| window.document()) {
                        if document.fullscreen_element().is_some() {
                            document.exit_fullscreen();
                        } else if let Some(element) = document.document_element() {
                            let _ = element.request_fullscreen();
                        }
                    }
                },
                FullscreenIcon {}
            }
            ModalContainer {}
            Field {}
            Hand {}
        }
    )
}
