mod banishment;
mod card_interaction;
mod card_stats;
mod constants;
mod extra_deck;
mod field;
mod field_preview;
mod graveyard;
mod hand;
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
use crate::ui::card_playback::{CardPlayback, wait_for_animation};
use crate::ui::card_tracking::CardTracking;
use crate::ui::components::UIButton;
use crate::ui::components::svg::FullscreenIcon;
use crate::ui::components::svg::ResetIcon;
use field_preview::FieldPreview;
use modal::ModalContainer;

#[component]
pub fn DuelWrapper(
    core_resource: Resource<anyhow::Result<OCGCore>>,
    custom_hand: Option<String>,
) -> Element {
    let mut state = use_context_provider(UIState::default);
    let mut duel_context = use_context_provider(|| Signal::new(None::<Duel>));
    let mut tracking = use_context_provider(|| Signal::new(None::<CardTracking>));

    let duel_resource = use_resource(move || {
        let custom_hand = custom_hand.clone();

        async move { load_duel(core_resource, custom_hand).await }
    });

    use_effect(move || {
        let next_setup = match &*duel_resource.read() {
            Some(Ok(setup)) => Some(setup.clone()),
            _ => None,
        };
        install_duel(&mut state, &mut duel_context, &mut tracking, next_setup);
    });

    match &*duel_resource.read() {
        Some(Ok(setup))
            if duel_context()
                .as_ref()
                .is_some_and(|duel| duel.ptr_eq(&setup.duel)) =>
        {
            rsx!(DuelScreen {
                duel_resource: duel_resource,
            })
        }
        Some(Ok(_)) => rsx!(LoadingScreen {}),
        Some(Err(e)) => rsx!("{e:#?}"),
        None => rsx!(LoadingScreen {}),
    }
}

fn install_duel(
    state: &mut UIState,
    duel_context: &mut Signal<Option<Duel>>,
    tracking: &mut Signal<Option<CardTracking>>,
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
    let (duel, cards) = match setup {
        Some(setup) => (Some(setup.duel), Some(CardTracking::Active(setup.cards))),
        None => (None, None),
    };
    tracking.set(cards);
    duel_context.set(duel);
}

#[component]
fn DuelScreen(duel_resource: Resource<anyhow::Result<DuelSetup>>) -> Element {
    let mut state = use_context::<UIState>();
    let tracking = use_context::<Signal<Option<CardTracking>>>();
    use_context_provider(layout_anchors::LayoutAnchors::default);
    let revision = use_context_provider(|| layout_anchors::LayoutRevision(Signal::new(0)));
    let mut layered_preview = use_signal(|| true);
    let startup_ready = use_signal(|| false);
    let mut playback = use_context_provider(|| {
        Signal::new(CardPlayback {
            enabled: *layered_preview.peek(),
            ..Default::default()
        })
    });

    use_effect(move || {
        if playback().enabled && playback.peek().failure.is_none() {
            if let Some(CardTracking::Failed(error)) = &*tracking.read() {
                playback.write().fail(error.clone());
            }
        }
    });

    use_effect(move || {
        let current = playback();
        if current.active {
            spawn(async move {
                wait_for_animation().await;
                if playback.peek().active && playback.peek().generation == current.generation {
                    playback.write().finish(current.generation);
                }
            });
        }
    });

    // Start game loop
    use_effect(run_game_loop);

    let loading = !startup_ready() && playback().failure.is_none();
    rsx!(
        main {
            class: "portrait:hidden relative h-dvh w-dvw bg-gray-800 select-none",
            oncontextmenu: handle_right_click,
            onresize: move |_| {
                revision.invalidate();
                if playback.peek().active {
                    playback.write().resize();
                }
            },
            onclick: move |_| {
                if !playback.peek().blocked() && (state.selected_card)().is_some() {
                    state.selected_card.set(None);
                }

            },
            InteractionGate {
                active: loading,
            UIButton {
                class: "fixed top-3 left-3 z-50 w-max h-max p-2",
                label: "Restart game",
                onclick: move |_| {
                    playback.write().restart();
                    state.reset();
                    duel_resource.clear();
                    duel_resource.restart();
                },
                ResetIcon {}
            }
            button {
                class: "fixed top-3 left-16 z-50 rounded-md border border-white/20 bg-black/70 px-3 py-2 text-xs font-semibold text-white cursor-pointer hover:bg-black/85",
                "aria-pressed": "{layered_preview}",
                disabled: playback().failure.is_some() || playback().restarting,
                onclick: move |event| {
                    event.stop_propagation();
                    layered_preview.toggle();
                    playback.write().set_enabled(layered_preview());
                },
                if layered_preview() { "Layered field: on" } else { "Layered field: off" }
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
            div {
                class: if loading || playback().blocked() { "hidden" },
                ModalContainer {}
            }
            InteractionGate {
                active: playback().blocked(),
                FieldPreview { enabled: layered_preview, startup_ready }
            }
            MotionPreference {}
            if let Some(error) = &playback().failure {
                div {
                    class: "fixed top-3 left-1/2 z-50 max-w-lg -translate-x-1/2 rounded bg-red-950 p-3 text-sm text-white shadow-lg",
                    role: "alert",
                    "Card playback stopped: {error}. Restart the duel to continue."
                }
            } else if let Some(CardTracking::Failed(error)) = &*tracking.read() {
                div {
                    class: "fixed top-3 left-1/2 z-50 max-w-lg -translate-x-1/2 rounded bg-red-950 p-3 text-sm text-white shadow-lg",
                    role: "alert",
                    "Card tracking stopped: {error}"
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
fn MotionPreference() -> Element {
    let mut playback = use_context::<Signal<CardPlayback>>();
    let mut revision = use_signal(|| 0u64);
    rsx! {
        div {
            class: "absolute left-0 top-0 w-px h-px motion-reduce:w-[2px] opacity-0 pointer-events-none",
            "aria-hidden": "true",
            onmounted: move |event| {
                let element = event.data();
                let measured_revision = *revision.peek();
                spawn(async move {
                    match element.get_client_rect().await {
                        Ok(rect) if *revision.peek() == measured_revision => {
                            set_motion_width(&mut playback, rect.size.width);
                        }
                        Ok(_) => {}
                        Err(error) => tracing::warn!("Cannot measure reduced-motion preference: {error}"),
                    }
                });
            },
            onresize: move |event| {
                revision += 1;
                match event.data().get_border_box_size() {
                    Ok(size) => set_motion_width(&mut playback, size.width),
                    Err(error) => {
                        tracing::warn!("Cannot observe reduced-motion preference: {error}");
                        if !playback.peek().reduced_motion {
                            playback.write().set_reduced_motion(true);
                        }
                    }
                }
            },
        }
    }
}

fn set_motion_width(playback: &mut Signal<CardPlayback>, width: f64) {
    let reduced = if width.is_finite() && width > 0.0 {
        width > 1.5
    } else {
        tracing::warn!("Invalid reduced-motion probe width: {width}");
        true
    };
    if playback.peek().reduced_motion != reduced {
        playback.write().set_reduced_motion(reduced);
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
