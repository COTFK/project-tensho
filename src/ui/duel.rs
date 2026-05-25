use dioxus::prelude::*;

use super::buttons::FullscreenButton;
use super::buttons::ResetButton;
use super::field::Field;
use super::hand::Hand;
use super::modal::ModalContainer;
use crate::ocgcore::Duel;
use crate::state::DuelState;
use crate::state::handle_left_click;
use crate::state::handle_right_click;
use crate::state::run_game_loop;
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
            ResetButton {
                onclick: move |_| {
                    resource_handle.clear();
                    resource_handle.restart();
                }
            }
            FullscreenButton {}
            ModalContainer {}
            Field {}
            Hand {}
        }
    )
}
