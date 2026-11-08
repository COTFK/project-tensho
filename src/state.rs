use dioxus::prelude::*;
use std::future::pending;

use crate::define_ui_state;
use crate::duel::cards::{CardAddress, CardTracker};
use crate::duel::controller::{
    self, DuelController, DuelSetup, process_until_blocked, resolve_chain_event,
};
use crate::duel::input::{CardActionOptions, CardIntent, PendingInput};
use crate::ocgcore::OCGCore;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::Phase;
use crate::ocgcore::messages::CoreMessage;
use crate::ui::card_animation::plan_animation;
use crate::ui::playback::DuelPlayback;
use crate::ui::view_state::DuelViewState;
use ocgcore_ffi::enums::OCG_DuelStatus;

pub(crate) fn current_card_action_options(
    state: &DuelState,
    view: &DuelViewState,
    address: CardAddress,
) -> Option<CardActionOptions> {
    if !view.is_selected(address, state.cards.peek().as_ref())
        || !view.effects_to_select_from.peek().is_empty()
    {
        tracing::warn!("Ignoring a stale or suppressed card action at {address:?}");
        return None;
    }
    Some(prompt_card_action_options(state, address))
}

fn prompt_card_action_options(state: &DuelState, address: CardAddress) -> CardActionOptions {
    state
        .pending_input
        .read()
        .as_ref()
        .map(|input| input.card_action_options(address))
        .unwrap_or_default()
}

pub(crate) fn handle_card_intent(
    mut state: DuelState,
    mut view: DuelViewState,
    playback: DuelPlayback,
    controller: ReadSignal<Option<DuelController>>,
    action: CardIntent,
) {
    if state.interaction_blocked(playback) {
        tracing::warn!("Ignoring card action while interaction is blocked");
        return;
    }
    if let Some(response) = apply_card_intent(&mut state, &mut view, action) {
        send_response(state, view, playback, controller, response);
    }
}

pub(crate) fn apply_card_intent(
    state: &mut DuelState,
    view: &mut DuelViewState,
    action: CardIntent,
) -> Option<Response> {
    let address = match action {
        CardIntent::Select(address)
        | CardIntent::Summon(address)
        | CardIntent::Activate(address)
        | CardIntent::Set(address)
        | CardIntent::SpecialSummon(address) => address,
    };
    if matches!(action, CardIntent::Select(_)) {
        let actions = prompt_card_action_options(state, address);
        if address.location != CardLocation::Hand {
            if let Some(index) = actions.tribute {
                state.pending_input.with_mut(|input| {
                    let Some(PendingInput::TributeSelection { selected, .. }) = input else {
                        tracing::warn!("Tribute selection is no longer available at {address:?}");
                        return;
                    };
                    if let Some(position) = selected.iter().position(|selected| *selected == index)
                    {
                        selected.remove(position);
                    } else if selected.len() < actions.tribute_max {
                        selected.push(index);
                    }
                });
                return None;
            }
            if let Some(index) = actions.selection {
                return Some(Response::SelectUnselectCard { index });
            }
        }
        view.select_card(address, state.cards.peek().as_ref());
        return None;
    }
    let pile = matches!(
        address.location,
        CardLocation::ExtraDeck | CardLocation::Graveyard | CardLocation::Banishment
    );
    let actions = if pile {
        if *view.open_pile.peek() != Some(address.location)
            || !view.effects_to_select_from.peek().is_empty()
        {
            tracing::warn!("Ignoring a stale or suppressed pile action at {address:?}");
            return None;
        }
        prompt_card_action_options(state, address)
    } else {
        let Some(actions) = current_card_action_options(state, view, address) else {
            return None;
        };
        actions
    };
    match action {
        CardIntent::Summon(_) => {
            let Some(index) = actions.summon else {
                tracing::warn!("Summon is no longer available at {address:?}");
                return None;
            };
            view.selected_card.set(None);
            Some(Response::NormalSummon { index })
        }
        CardIntent::Set(_) => {
            if let Some(index) = actions.spell_trap_set {
                view.selected_card.set(None);
                Some(Response::SetSpellTrap { index })
            } else if let Some(index) = actions.monster_set {
                view.selected_card.set(None);
                Some(Response::SetMonster { index })
            } else {
                tracing::warn!("Setting is no longer available at {address:?}");
                None
            }
        }
        CardIntent::Activate(_) => {
            let hand = address.location == CardLocation::Hand;
            if let Some(prompted) = actions.prompted {
                if let Some(index) = prompted.action_index {
                    view.selected_card.set(None);
                    return Some(Response::Chain { index });
                }
                if !hand {
                    return Some(Response::Yes);
                }
            }
            if !pile && !hand && actions.effects.len() > 1 {
                view.effects_to_select_from.set(actions.effects);
                view.selected_card.set(None);
                None
            } else if !pile
                && let Some(index) = actions
                    .effects
                    .last()
                    .and_then(|effect| effect.action_index)
            {
                view.selected_card.set(None);
                Some(Response::Activate { index })
            } else {
                tracing::warn!("Activation is no longer available at {address:?}");
                None
            }
        }
        CardIntent::SpecialSummon(_) => {
            let Some(index) = actions.special_summon else {
                tracing::warn!("Special summon is no longer available at {address:?}");
                return None;
            };
            Some(Response::SpecialSummon { index })
        }
        CardIntent::Select(_) => unreachable!(),
    }
}

define_ui_state!(DuelState {
    cards: Option<CardTracker>,
    current_phase: Option<Phase>,
    pending_input: Option<PendingInput>,
    duel_finished: bool,
    restarting: bool,
    failure: Option<String>,
});

pub(crate) fn process_duel_until_blocked(
    mut state: DuelState,
    mut view: DuelViewState,
    mut playback: DuelPlayback,
    controller: ReadSignal<Option<DuelController>>,
) {
    let tracking = state.cards;
    if state.processing_blocked(playback) {
        return;
    }
    let controller = controller().expect("Duel context missing active controller");
    let mut duel_finished = state.duel_finished;
    process_until_blocked(
        || {
            let status = controller.process();
            if status == OCG_DuelStatus::OCG_DUEL_STATUS_END {
                duel_finished.set(true);
            }
            status
        },
        || {
            let before = tracking.peek().as_ref().cloned();
            let messages = controller.parse_messages();
            let waiting =
                apply_core_message_batch(&mut state, &mut view, &controller, &messages, playback);
            if (state.restarting)() || (state.failure)().is_some() {
                return true;
            }
            if let Some(before) = before {
                if let Some(after) = &*tracking.peek() {
                    if let Some(animation) = plan_animation(&messages, &before, after) {
                        animation.begin(&mut playback.animation.write(), before);
                        return true;
                    }
                }
            }
            waiting || state.interaction_blocked(playback)
        },
    );
}

pub(crate) fn handle_right_click(
    state: DuelState,
    mut view: DuelViewState,
    playback: DuelPlayback,
    controller: ReadSignal<Option<DuelController>>,
    evt: MouseEvent,
) {
    evt.prevent_default();
    if state.interaction_blocked(playback) {
        return;
    }

    let response = state
        .pending_input
        .peek()
        .as_ref()
        .and_then(PendingInput::decline_response);
    if let Some(response) = response {
        send_response(state, view, playback, controller, response);
    }

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

pub(crate) fn send_response(
    mut state: DuelState,
    mut view: DuelViewState,
    playback: DuelPlayback,
    controller: ReadSignal<Option<DuelController>>,
    response: Response,
) {
    if state.interaction_blocked(playback) {
        tracing::warn!("Ignoring response during card animation");
        return;
    }
    if state.pending_input.peek().is_none() {
        tracing::warn!("Ignoring response without a pending prompt");
        return;
    }
    let controller = controller().expect("Duel context missing active controller");

    controller.send_response(response);
    state.clear_pending_input();
    view.reset();
}

impl DuelState {
    pub(crate) fn waiting_on_input(&self) -> bool {
        self.pending_input.read().is_some()
    }

    pub(crate) fn interaction_blocked(&self, playback: DuelPlayback) -> bool {
        (self.restarting)() || (self.failure)().is_some() || playback.is_active()
    }

    pub(crate) fn processing_blocked(&self, playback: DuelPlayback) -> bool {
        self.interaction_blocked(playback) || (self.duel_finished)() || self.waiting_on_input()
    }

    pub(crate) fn modals_suppressed(&self, startup_ready: bool, playback: DuelPlayback) -> bool {
        (!startup_ready && (self.failure)().is_none()) || self.interaction_blocked(playback)
    }

    pub(crate) fn reset_duel(&mut self, view: &mut DuelViewState, mut playback: DuelPlayback) {
        playback.cancel();
        self.reset();
        view.reset();
    }

    pub(crate) fn restart(&mut self, view: &mut DuelViewState, playback: DuelPlayback) {
        self.reset_duel(view, playback);
        self.restarting.set(true);
    }

    pub(crate) fn fail(&mut self, error: String, mut playback: DuelPlayback) {
        if self.failure.peek().is_none() {
            playback.cancel();
            self.failure.set(Some(error));
        }
    }

    fn tracking_failed(&mut self, view: &mut DuelViewState, error: String, playback: DuelPlayback) {
        tracing::error!("{error}");
        self.cards.set(None);
        view.selected_card.set(None);
        self.fail(error, playback);
    }

    fn apply_card_message(
        &mut self,
        view: &mut DuelViewState,
        message: &CoreMessage,
        playback: DuelPlayback,
    ) -> bool {
        if self.interaction_blocked(playback) {
            return false;
        }
        let mut cards = self.cards;
        let result = cards.with_mut(|tracking| {
            let tracker = tracking
                .as_mut()
                .expect("Card tracking missing for active duel");
            tracker.apply(message)?;
            view.reconcile_selection(tracker);
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

    pub fn clear_pending_input(&mut self) {
        self.pending_input.set(None);
    }
}

fn apply_core_message_batch(
    state: &mut DuelState,
    view: &mut DuelViewState,
    controller: &DuelController,
    messages: &[CoreMessage],
    mut playback: DuelPlayback,
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
        {
            let cards = tracking.peek();
            let tracker = cards
                .as_ref()
                .expect("Card tracking missing for active duel");
            if let Some(event) = resolve_chain_event(&message, tracker) {
                playback.chain.write().record(event);
            }
        }
        let responded = pending_input.with_mut(|input| {
            current_phase.with_mut(|phase| controller.handle_message(message, input, phase))
        });
        if responded {
            view.reset();
        }
    }

    let result = tracking.with_mut(|tracking| {
        let tracker = tracking
            .as_mut()
            .expect("Card tracking missing for active duel");
        controller.refresh_queried_card_data(tracker)?;
        view.reconcile_selection(tracker);
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

    if playback.chain.peek().has_pending_frames() {
        playback.chain.write().start();
    }
    state.waiting_on_input()
}

#[cfg(test)]
#[path = "../tests/unit/state.rs"]
mod tests;
