mod banishment;
mod constants;
mod extra_deck;
mod field;
mod graveyard;
mod hand;
mod main_deck;
mod modal;

pub use hand::Hand;

use dioxus::prelude::*;
use web_sys::window;

use crate::ocgcore::CardData;
use crate::ocgcore::Duel;
use crate::ocgcore::OCGCore;
use crate::state::UIState;
use crate::state::handle_right_click;
use crate::state::load_duel;
use crate::state::run_game_loop;
use crate::ui::Animator;
use crate::ui::LoadingScreen;
use crate::ui::animation::{
    ANIMATION_CONTROLLER, AnimationRequest, AnimationTrack, DrawCard, DrawToHand, cancel_animations,
};
use crate::ui::components::UIButton;
use crate::ui::components::svg::FullscreenIcon;
use crate::ui::components::svg::ResetIcon;
use field::Field;
use modal::ModalContainer;

pub fn start_draw_animation(cards: Vec<CardData>, hand_size: usize, opening: bool) {
    if cards.is_empty() {
        return;
    }
    let Some(first_drawn_index) = hand_size.checked_sub(cards.len()) else {
        tracing::error!(
            hand_size,
            draw_count = cards.len(),
            "Draw batch exceeds the current hand"
        );
        return;
    };
    let tracks = cards
        .into_iter()
        .zip(first_drawn_index..hand_size)
        .enumerate()
        .map(|(draw_index, (card, target_index))| {
            let (rotation, translate_y) = hand::hand_transform(target_index, hand_size);
            let animation = DrawToHand::new(rotation, translate_y, draw_index, opening);
            let delay_ms = animation.delay_ms;
            AnimationTrack::between(
                rsx!(DrawCard { card }),
                Box::new(animation),
                "main-deck-animation-source",
                format!("hand-animation-target-{target_index}"),
            )
            .delayed(delay_ms)
            .preserve_source()
        })
        .collect();

    ANIMATION_CONTROLLER.with_mut(|controller| controller.enqueue(AnimationRequest::new(tracks)));
}

#[component]
pub fn DuelWrapper(
    core_resource: Resource<anyhow::Result<OCGCore>>,
    custom_hand: Option<String>,
) -> Element {
    let mut state = use_context_provider(UIState::default);
    let mut duel_context = use_context_provider(|| Signal::new(None::<Duel>));

    let duel_resource = use_resource(move || {
        let custom_hand = custom_hand.clone();

        async move { load_duel(core_resource, custom_hand).await }
    });

    use_effect(move || {
        let next_duel = match &*duel_resource.read() {
            Some(Ok(duel)) => Some(duel.clone()),
            _ => None,
        };

        let current_duel = duel_context();
        let unchanged = match (&current_duel, &next_duel) {
            (Some(current), Some(next)) => current.ptr_eq(next),
            (None, None) => true,
            _ => false,
        };
        if unchanged {
            return;
        }

        cancel_animations();
        state.reset();
        duel_context.set(next_duel);
    });

    match &*duel_resource.read() {
        Some(Ok(_)) => rsx!(DuelScreen {
            duel_resource: duel_resource,
        }),
        Some(Err(e)) => rsx!("{e:#?}"),
        None => rsx!(LoadingScreen {}),
    }
}

#[component]
pub fn DuelScreen(duel_resource: Resource<anyhow::Result<Duel>>) -> Element {
    let mut state = use_context::<UIState>();

    // Start game loop
    use_effect(run_game_loop);

    rsx!(
        main {
            class: "portrait:hidden relative h-dvh w-dvw bg-gray-800 select-none",
            oncontextmenu: handle_right_click,
            onclick: move |_| {
                if (state.selected_card)().is_some() {
                    state.selected_card.set(None);
                }
            },
            UIButton {
                class: "fixed top-3 left-3 z-50 w-max h-max p-2",
                label: "Restart game",
                onclick: move |_| {
                    cancel_animations();
                    state.reset();
                    duel_resource.clear();
                    duel_resource.restart();
                },
                ResetIcon {}
            }
            UIButton {
                label: "Toggle fullscreen",
                class: "fixed top-3 right-3 z-50 w-max h-max p-2",
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
            Animator {}
            ModalContainer {}
            Field {}
            Hand {}
        }
    )
}
