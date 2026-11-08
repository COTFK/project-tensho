mod card_interaction;
mod card_stats;
mod chain_badges;
mod constants;
mod effect_pile;
mod extra_deck;
mod field;
mod hand;
mod layered_field;
mod layout_anchors;
mod main_deck;
mod modal;
mod persistent_card_layer;

use dioxus::prelude::*;
use web_sys::window;

use crate::ocgcore::Duel;
use crate::ocgcore::OCGCore;
use crate::state::DuelSetup;
use crate::state::UIState;
use crate::state::handle_right_click;
use crate::state::load_duel;
use crate::state::run_game_loop;
use crate::ui::LoadingScreen;
use crate::ui::card_animation::{CardAnimationState, use_animation_completion};
use crate::ui::chain::{ChainPlayback, use_chain_playback};
use crate::ui::components::ModalSuppression;
use crate::ui::components::UIButton;
use crate::ui::components::svg::FullscreenIcon;
use crate::ui::components::svg::ResetIcon;
use crate::ui::view_state::ViewState;
use layered_field::LayeredField;
use modal::ModalContainer;

#[component]
pub fn DuelWrapper(
    core_resource: Resource<anyhow::Result<OCGCore>>,
    custom_hand: Option<String>,
) -> Element {
    let mut state = use_context_provider(UIState::default);
    let mut view = use_context_provider(ViewState::default);
    let mut duel_context = use_context_provider(|| Signal::new(None::<Duel>));

    let duel_resource = use_resource(move || {
        let custom_hand = custom_hand.clone();

        async move { load_duel(core_resource, custom_hand).await }
    });

    use_effect(move || {
        let next_setup = match &*duel_resource.read() {
            Some(Ok(setup)) => Some(setup.clone()),
            _ => None,
        };
        install_duel(&mut state, &mut view, &mut duel_context, next_setup);
    });

    match &*duel_resource.read() {
        Some(Ok(setup))
            if duel_context()
                .as_ref()
                .is_some_and(|duel| duel.ptr_eq(&setup.duel)) =>
        {
            rsx!(DuelScreen {
                duel_resource: duel_resource,
                deck_codes: setup
                    .cards
                    .cards()
                    .iter()
                    .map(|card| card.code)
                    .collect::<Vec<_>>(),
            })
        }
        Some(Ok(_)) => rsx!(LoadingScreen {}),
        Some(Err(e)) => rsx!("{e:#?}"),
        None => rsx!(LoadingScreen {}),
    }
}

fn install_duel(
    state: &mut UIState,
    view: &mut ViewState,
    duel_context: &mut Signal<Option<Duel>>,
    setup: Option<DuelSetup>,
) {
    let unchanged = match (&*duel_context.peek(), &setup) {
        (Some(current), Some(next)) => current.ptr_eq(&next.duel),
        (None, None) => true,
        _ => false,
    };
    if unchanged {
        return;
    }
    state.reset();
    view.reset();
    let (duel, cards) = match setup {
        Some(setup) => (Some(setup.duel), Some(setup.cards)),
        None => (None, None),
    };
    state.cards.set(cards);
    duel_context.set(duel);
}

#[component]
fn DuelScreen(duel_resource: Resource<anyhow::Result<DuelSetup>>, deck_codes: Vec<u32>) -> Element {
    let mut state = use_context::<UIState>();
    let mut view = use_context::<ViewState>();
    use_context_provider(layout_anchors::LayoutAnchors::default);
    let revision = use_context_provider(|| layout_anchors::LayoutRevision(Signal::new(0)));
    let startup_ready = use_signal(|| false);
    let mut playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
    let mut chain = use_context_provider(|| Signal::new(ChainPlayback::default()));
    let modal_suppressed = use_memo(move || {
        (!startup_ready() && (state.failure)().is_none())
            || state.interaction_blocked(Some(playback))
    });
    use_context_provider(|| ModalSuppression(modal_suppressed));

    use_animation_completion(playback);
    use_chain_playback(chain);

    // Start game loop
    use_effect(run_game_loop);

    let loading = !startup_ready() && (state.failure)().is_none();
    rsx!(
        main {
            class: "portrait:hidden relative h-dvh w-dvw bg-gray-800 select-none",
            oncontextmenu: handle_right_click,
            onresize: move |_| revision.invalidate(),
            onclick: move |_| {
                if !state.interaction_blocked(Some(playback)) && (view.selected_card)().is_some() {
                    view.selected_card.set(None);
                }

            },
            InteractionGate {
                active: loading,
            UIButton {
                class: "fixed top-3 left-3 z-50 w-max h-max p-2",
                label: "Restart game",
                onclick: move |_| {
                    playback.write().cancel();
                    chain.write().cancel();
                    state.reset();
                    view.reset();
                    state.restarting.set(true);
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
            InteractionGate {
                active: modal_suppressed(),
                ModalContainer {}
            }
            InteractionGate {
                active: state.interaction_blocked(Some(playback)),
                LayeredField { startup_ready, deck_codes }
            }
            if let Some(error) = &(state.failure)() {
                div {
                    class: "fixed top-3 left-1/2 z-50 max-w-lg -translate-x-1/2 rounded bg-red-950 p-3 text-sm text-white shadow-lg",
                    role: "alert",
                    "Duel stopped: {error}. Restart the duel to continue."
                }
            }
            if let Some(phase) = (state.current_phase)() {
                div {
                    class: "fixed bottom-3 right-3 z-50 rounded-md bg-gray-950/70 px-2 py-1 font-mono text-gray-200 text-[8px] md:text-xs lg:text-base shadow-lg ring-1 ring-white/10 backdrop-blur-sm pointer-events-none",
                    title: "Current phase",
                    "{phase}"
                }
            }
            }
            if loading {
                div {
                    class: "fixed inset-0 z-[100]",
                    LoadingScreen {}
                }
            }
        }
    )
}

#[component]
fn InteractionGate(active: bool, children: Element) -> Element {
    rsx! {
        div {
            inert: if active { Some("") } else { None },
            class: if active { "pointer-events-none" },
            {children}
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/duel.rs"]
mod tests;
