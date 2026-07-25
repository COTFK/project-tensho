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

use crate::ocgcore::Duel;
use crate::ocgcore::OCGCore;
use crate::ocgcore::Response;
use crate::state::UIState;
use crate::state::handle_right_click;
use crate::state::load_duel;
use crate::state::run_game_loop;
use crate::state::send_response;
use crate::ui::Animator;
use crate::ui::LoadingScreen;
use crate::ui::animation::{AnimationBounds, AnimationStatus, CURRENT_ANIMATION};
use crate::ui::components::UIButton;
use crate::ui::components::svg::FullscreenIcon;
use crate::ui::components::svg::ResetIcon;
use field::Field;
use modal::ModalContainer;

pub(super) struct PendingSummon {
    pub source_hand_index: usize,
    pub placement: Option<Response>,
}

pub(super) static PENDING_SUMMON: GlobalSignal<Option<PendingSummon>> = Signal::global(|| None);

pub(super) fn start_pending_summon(destination: AnimationBounds, placement: Response) -> bool {
    CURRENT_ANIMATION.with_mut(|animation| {
        if let (Some(animation), Some(pending_summon)) =
            (animation.as_mut(), PENDING_SUMMON.write().as_mut())
        {
            if animation.status == AnimationStatus::Queued {
                pending_summon.placement = Some(placement);
                animation.start(destination);
                return true;
            }
        }

        false
    })
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
        if current_duel == next_duel {
            return;
        }

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
    use_effect(move || {
        let animation_complete = CURRENT_ANIMATION
            .read()
            .as_ref()
            .is_some_and(|animation| animation.status == AnimationStatus::Done);

        if !animation_complete {
            return;
        }

        let response = PENDING_SUMMON.with_mut(|pending_summon| {
            pending_summon
                .as_mut()
                .and_then(|pending_summon| pending_summon.placement.take())
        });

        if let Some(response) = response {
            *PENDING_SUMMON.write() = None;
            *CURRENT_ANIMATION.write() = None;
            send_response(response);
        }
    });

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
                    *CURRENT_ANIMATION.write() = None;
                    *PENDING_SUMMON.write() = None;
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
