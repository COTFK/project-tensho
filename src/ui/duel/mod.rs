mod card_interaction;
mod card_layout;
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
mod pile_cards;

use dioxus::prelude::*;
use web_sys::window;

use crate::duel::controller::{DuelController, DuelSetup};
use crate::ocgcore::OCGCore;
use crate::state::UIState;
use crate::state::handle_right_click;
use crate::state::load_duel;
use crate::state::run_game_loop;
use crate::state::{handle_card_action, send_response};
use crate::ui::LoadingScreen;
use crate::ui::card_animation::use_animation_completion;
use crate::ui::chain::use_chain_playback;
use crate::ui::components::ModalSuppression;
use crate::ui::components::UIButton;
use crate::ui::components::svg::FullscreenIcon;
use crate::ui::components::svg::ResetIcon;
use crate::ui::playback::Playback;
use crate::ui::view_state::ViewState;
use layered_field::{LayeredField, LayoutStatus};
use modal::ModalContainer;

#[component]
pub fn DuelWrapper(
    core_resource: Resource<anyhow::Result<OCGCore>>,
    custom_hand: Option<String>,
) -> Element {
    let mut state = use_hook(UIState::default);
    let mut view = use_hook(ViewState::default);
    let playback = use_hook(Playback::default);
    let mut controller = use_signal(|| None::<DuelController>);
    let controller_input = use_hook(move || ReadSignal::new(controller));
    use_duel_contexts(state, view, playback, controller_input);

    let duel_resource = use_resource(move || {
        let custom_hand = custom_hand.clone();

        async move { load_duel(core_resource, custom_hand).await }
    });

    use_effect(move || {
        let next_setup = match &*duel_resource.read() {
            Some(Ok(setup)) => Some(setup.clone()),
            _ => None,
        };
        install_duel(&mut state, &mut view, playback, &mut controller, next_setup);
    });

    match &*duel_resource.read() {
        Some(Ok(setup))
            if controller()
                .as_ref()
                .is_some_and(|duel| duel.ptr_eq(&setup.controller)) =>
        {
            rsx!(DuelScreen {
                state,
                view,
                playback,
                controller: controller_input,
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

fn use_duel_contexts(
    state: UIState,
    view: ViewState,
    playback: Playback,
    controller: ReadSignal<Option<DuelController>>,
) {
    use_context_provider(|| ReadSignal::new(state.cards));
    use_context_provider(|| ReadSignal::new(state.pending_input));
    use_context_provider(|| view.open_pile);
    use_context_provider(|| view.effects_to_select_from);
    use_context_provider(|| {
        EventHandler::new(move |response| {
            send_response(state, view, playback, controller, response)
        })
    });
    use_context_provider(|| {
        EventHandler::new(move |action| {
            handle_card_action(state, view, playback, controller, action)
        })
    });
    let links = use_memo(move || playback.chain.read().displayed.clone());
    use_context_provider(|| links);
}

fn install_duel(
    state: &mut UIState,
    view: &mut ViewState,
    playback: Playback,
    controller: &mut Signal<Option<DuelController>>,
    setup: Option<DuelSetup>,
) {
    let unchanged = match (&*controller.peek(), &setup) {
        (Some(current), Some(next)) => current.ptr_eq(&next.controller),
        (None, None) => true,
        _ => false,
    };
    if unchanged {
        return;
    }
    state.reset_duel(view, playback);
    let (duel, cards) = match setup {
        Some(setup) => (Some(setup.controller), Some(setup.cards)),
        None => (None, None),
    };
    state.cards.set(cards);
    controller.set(duel);
}

#[component]
fn DuelScreen(
    mut state: UIState,
    mut view: ViewState,
    playback: Playback,
    controller: ReadSignal<Option<DuelController>>,
    duel_resource: Resource<anyhow::Result<DuelSetup>>,
    deck_codes: Vec<u32>,
) -> Element {
    use_context_provider(layout_anchors::LayoutAnchors::default);
    let revision = use_context_provider(|| layout_anchors::LayoutRevision(Signal::new(0)));
    let mut startup_ready = use_signal(|| false);
    let mut layout_status = use_signal(LayoutStatus::default);
    let modal_suppressed = use_memo(move || state.modals_suppressed(startup_ready(), playback));
    use_context_provider(|| ModalSuppression(modal_suppressed));

    use_animation_completion(playback.animation);
    use_chain_playback(playback.chain);

    use_effect(move || run_game_loop(state, view, playback, controller));
    use_effect(move || {
        synchronize_layout(&mut state, &mut startup_ready, playback, layout_status());
    });

    let loading = !startup_ready() && (state.failure)().is_none();
    rsx!(
        main {
            class: "portrait:hidden relative h-dvh w-dvw bg-gray-800 select-none",
            oncontextmenu: move |event| handle_right_click(state, view, playback, controller, event),
            onresize: move |_| revision.invalidate(),
            onclick: move |_| {
                if !state.interaction_blocked(playback) && (view.selected_card)().is_some() {
                    view.selected_card.set(None);
                }

            },
            InteractionGate {
                active: loading,
            UIButton {
                class: "fixed top-3 left-3 z-50 w-max h-max p-2",
                label: "Restart game",
                onclick: move |_| {
                    state.restart(&mut view, playback);
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
                active: state.interaction_blocked(playback),
                LayeredField {
                    tracking: state.cards,
                    selected: view.selected_card,
                    animation: playback.animation,
                    preloading: !startup_ready(),
                    effects_open: !view.effects_to_select_from.read().is_empty(),
                    deck_codes,
                    onlayout: move |status| {
                        if *layout_status.peek() != status {
                            layout_status.set(status);
                        }
                    },
                }
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

fn synchronize_layout(
    state: &mut UIState,
    startup_ready: &mut Signal<bool>,
    playback: Playback,
    layout: LayoutStatus,
) {
    if (state.failure)().is_some() || (state.restarting)() {
        return;
    }
    match layout {
        LayoutStatus::Failed(error) => {
            tracing::error!("{error}");
            state.fail(error, playback);
        }
        LayoutStatus::Ready
            if !startup_ready()
                && (state.waiting_on_input() || (state.duel_finished)())
                && !playback.is_active() =>
        {
            startup_ready.set(true);
        }
        LayoutStatus::Pending | LayoutStatus::Ready => {}
    }
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
