use dioxus::prelude::*;
use std::future::pending;

use crate::define_ui_state;
use crate::duel::cards::{CardAddress, CardTracker};
use crate::duel::controller::{self, DuelController, DuelSetup, process_until_input};
use crate::duel::input::PendingInput;
use crate::ocgcore::CardData;
use crate::ocgcore::OCGCore;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::Phase;
use crate::ocgcore::messages::CoreMessage;
use crate::ui::card_animation::{CardAnimationState, plan_animation};
use crate::ui::chain::ChainPlayback;
use crate::ui::view_state::ViewState;
use ocgcore_ffi::enums::OCG_DuelStatus;

pub(crate) fn select_card(address: CardAddress) {
    let state = consume_context::<UIState>();
    if state.interaction_blocked(try_consume_context::<Signal<CardAnimationState>>()) {
        tracing::warn!("Ignoring card selection during animation");
        return;
    }
    let tracking = state.cards;
    let card_id = match &*tracking.peek() {
        Some(layer) => match layer.card_at(address) {
            Some(card) => card.id,
            None => {
                tracing::error!("Cannot select untracked card at {address:?}");
                return;
            }
        },
        None => {
            tracing::error!("Cannot select a card without an active tracker");
            return;
        }
    };
    let mut view = consume_context::<ViewState>();
    view.selected_card.set(Some(card_id));
}

define_ui_state!(UIState {
    cards: Option<CardTracker>,
    current_phase: Option<Phase>,
    pending_input: Option<PendingInput>,
    duel_finished: bool,
    restarting: bool,
    failure: Option<String>,
});

pub fn run_game_loop() {
    let mut state = consume_context::<UIState>();
    let playback = try_consume_context::<Signal<CardAnimationState>>();
    let tracking = state.cards;
    if state.interaction_blocked(playback) || (state.duel_finished)() || state.waiting_on_input() {
        return;
    }
    let controller = consume_context::<Signal<Option<DuelController>>>();
    let controller = controller().expect("Duel context missing active controller");
    let mut view = consume_context::<ViewState>();
    let chain = try_consume_context::<Signal<ChainPlayback>>();
    let mut duel_finished = state.duel_finished;
    process_until_input(
        || {
            let status = controller.process();
            if status == OCG_DuelStatus::OCG_DUEL_STATUS_END {
                duel_finished.set(true);
            }
            status
        },
        || {
            let before = if playback.is_some() {
                match &*tracking.peek() {
                    Some(layer) => Some(layer.clone()),
                    _ => None,
                }
            } else {
                None
            };
            let messages = controller.parse_messages();
            let waiting = handle_core_messages(
                &mut state,
                &mut view,
                &controller,
                &messages,
                playback,
                chain,
            );
            if (state.restarting)() || (state.failure)().is_some() {
                return true;
            }
            if let (Some(before), Some(mut playback)) = (before, playback) {
                if let Some(after) = &*tracking.peek() {
                    if let Some(animation) = plan_animation(&messages, &before, after) {
                        animation.begin(&mut playback.write(), before);
                        return true;
                    }
                }
            }
            waiting || state.interaction_blocked(playback)
        },
    );
}

pub fn handle_right_click(evt: MouseEvent) {
    evt.prevent_default();
    let state = consume_context::<UIState>();
    if state.interaction_blocked(try_consume_context::<Signal<CardAnimationState>>()) {
        return;
    }

    let response = state
        .pending_input
        .peek()
        .as_ref()
        .and_then(PendingInput::decline_response);
    if let Some(response) = response {
        send_response(response);
    }

    let mut view = consume_context::<ViewState>();
    view.open_pile.set(None);
}

pub(crate) async fn load_duel(
    core_resource: Resource<anyhow::Result<OCGCore>>,
    custom_hand: Option<String>,
) -> anyhow::Result<DuelSetup> {
    let core = core_resource
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok().cloned());

    let core = match core {
        Some(core) => core,
        None => return pending().await,
    };

    controller::load_duel(core, custom_hand).await
}

pub fn send_response(response: Response) {
    let mut state = consume_context::<UIState>();
    if state.interaction_blocked(try_consume_context::<Signal<CardAnimationState>>()) {
        tracing::warn!("Ignoring response during card animation");
        return;
    }
    if state.pending_input.peek().is_none() {
        tracing::warn!("Ignoring response without a pending prompt");
        return;
    }
    let controller = consume_context::<Signal<Option<DuelController>>>();
    let controller = controller().expect("Duel context missing active controller");

    controller.send_response(response);
    state.clear_pending_input();
    consume_context::<ViewState>().reset();
}

impl UIState {
    pub(crate) fn waiting_on_input(&self) -> bool {
        self.pending_input.read().is_some()
    }

    pub(crate) fn interaction_blocked(&self, playback: Option<Signal<CardAnimationState>>) -> bool {
        (self.restarting)()
            || (self.failure)().is_some()
            || playback.is_some_and(|playback| playback().is_active())
            || try_consume_context::<Signal<ChainPlayback>>().is_some_and(|chain| chain().active)
    }

    pub(crate) fn fail(&mut self, error: String, playback: Option<Signal<CardAnimationState>>) {
        if self.failure.peek().is_none() {
            if let Some(mut playback) = playback {
                playback.write().cancel();
            }
            if let Some(mut chain) = try_consume_context::<Signal<ChainPlayback>>() {
                chain.write().cancel();
            }
            self.failure.set(Some(error));
        }
    }

    fn tracking_failed(
        &mut self,
        view: &mut ViewState,
        error: String,
        playback: Option<Signal<CardAnimationState>>,
    ) {
        tracing::error!("{error}");
        self.cards.set(None);
        view.selected_card.set(None);
        self.fail(error, playback);
    }

    fn apply_card_message(
        &mut self,
        view: &mut ViewState,
        message: &CoreMessage,
        playback: Option<Signal<CardAnimationState>>,
    ) -> bool {
        if self.interaction_blocked(playback) {
            return false;
        }
        let mut cards = self.cards;
        let result = cards.with_mut(|tracking| {
            let layer = tracking
                .as_mut()
                .expect("Card tracking missing for active duel");
            layer.apply(message)?;
            view.reconcile_selection(layer);
            Ok::<_, anyhow::Error>(())
        });
        if let Err(error) = result {
            self.tracking_failed(
                view,
                format!("Card tracking failed for {message:?}: {error:#}"),
                playback,
            );
            return false;
        }
        true
    }

    pub(crate) fn zone_data(&self, location: CardLocation) -> Option<Vec<Option<CardData>>> {
        match &*self.cards.read() {
            Some(cards) => cards.zone_data(location),
            _ => None,
        }
    }

    pub fn clear_pending_input(&mut self) {
        self.pending_input.set(None);
    }
}

fn handle_core_messages(
    state: &mut UIState,
    view: &mut ViewState,
    controller: &DuelController,
    messages: &[CoreMessage],
    playback: Option<Signal<CardAnimationState>>,
    chain: Option<Signal<ChainPlayback>>,
) -> bool {
    let mut tracking = state.cards;
    let mut pending_input = state.pending_input;
    let mut current_phase = state.current_phase;
    if state.interaction_blocked(playback) {
        return true;
    }

    for message in messages.iter().cloned() {
        tracing::trace!("Received core message: {message:?}");
        if !state.apply_card_message(view, &message, playback) {
            return true;
        }
        if matches!(
            message,
            CoreMessage::Chaining(_) | CoreMessage::ChainSolved(_) | CoreMessage::ChainEnd
        ) && let Some(mut chain) = chain
        {
            let cards = tracking.peek();
            let layer = cards
                .as_ref()
                .expect("Card tracking missing for active duel");
            chain.write().record(&message, layer);
        }
        let responded = pending_input.with_mut(|input| {
            current_phase.with_mut(|phase| controller.handle_message(message, input, phase))
        });
        if responded {
            view.reset();
        }
    }

    let result = tracking.with_mut(|tracking| {
        let layer = tracking
            .as_mut()
            .expect("Card tracking missing for active duel");
        controller.refresh_data(layer)?;
        view.reconcile_selection(layer);
        Ok::<_, anyhow::Error>(())
    });
    if let Err(error) = result {
        state.tracking_failed(
            view,
            format!("Cannot refresh card data: {error:#}"),
            playback,
        );
        return true;
    }

    if let Some(mut chain) = chain
        && chain.peek().has_pending_frames()
    {
        chain.write().start();
    }
    state.waiting_on_input()
}

#[cfg(test)]
#[path = "../tests/unit/state.rs"]
mod tests;
