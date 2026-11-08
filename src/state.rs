use anyhow::Context;
use dioxus::prelude::*;
use rand::seq::SliceRandom;
use std::future::pending;

use crate::define_ui_state;
use crate::duel::cards::{CardAddress, CardPlacement, CardTracker};
use crate::duel::input::PendingInput;
use crate::ocgcore::CardData;
use crate::ocgcore::Duel;
use crate::ocgcore::OCGCore;
use crate::ocgcore::Response;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::Phase;
use crate::ocgcore::constants::Player;
use crate::ocgcore::log_core_message;
use crate::ocgcore::messages::CoreMessage;
use crate::settings::CustomHand;
use crate::ui::card_animation::{CardAnimationState, plan_animation};
use crate::ui::chain::ChainPlayback;
use crate::ui::view_state::ViewState;
use crate::utility::ARCHETYPE_SCRIPT;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::Scripts;
use crate::utility::cache_scripts;
use crate::utility::fetch_card_data;
use crate::utility::get_card_data;
use crate::utility::get_card_label;
use crate::utility::store_card_data;
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
    let state = consume_context::<UIState>();
    let playback = try_consume_context::<Signal<CardAnimationState>>();
    let tracking = state.cards;
    if state.interaction_blocked(playback) || (state.duel_finished)() || state.waiting_on_input() {
        return;
    }
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");
    let mut duel_finished = state.duel_finished;
    process_until_input(
        || {
            let status = duel.process();
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
            let messages = duel.parse_messages();
            let waiting = handle_core_messages(&messages);
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

fn process_until_input(
    mut process: impl FnMut() -> OCG_DuelStatus,
    mut handle_message: impl FnMut() -> bool,
) {
    loop {
        let status = process();
        let waiting = handle_message();
        if waiting || status == OCG_DuelStatus::OCG_DUEL_STATUS_END {
            break;
        }
    }
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

pub async fn cache_cards() -> anyhow::Result<Scripts> {
    let all_cards = MAIN_DECK_IDS
        .into_iter()
        .chain(EXTRA_DECK_IDS)
        .collect::<Vec<_>>();

    cache_scripts(&all_cards).await
}

fn pull_ids_to_front(main_array: &mut [u32; 40], target_ids: &[u32]) {
    let mut front_index = 0;

    for &target_id in target_ids {
        if let Some(pos) = main_array[front_index..]
            .iter()
            .position(|item| *item == target_id)
        {
            let actual_idx = front_index + pos;
            main_array[front_index..=actual_idx].rotate_right(1);

            front_index += 1;
        }
    }
}

pub async fn load_core(scripts: Scripts) -> anyhow::Result<OCGCore> {
    OCGCore::load(
        get_card_data,
        Box::new(move |name: &str| scripts.get(name).cloned()),
        log_core_message,
    )
    .await
}

#[derive(Clone)]
pub(crate) struct DuelSetup {
    pub duel: Duel,
    pub cards: CardTracker,
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

    let ids = MAIN_DECK_IDS
        .into_iter()
        .chain(EXTRA_DECK_IDS)
        .collect::<Vec<_>>();
    let archetype_script = core
        .script(ARCHETYPE_SCRIPT)
        .ok_or_else(|| anyhow::anyhow!("{ARCHETYPE_SCRIPT} is not loaded"))?;
    store_card_data(fetch_card_data(ids, &archetype_script).await?);

    let mut main_deck = MAIN_DECK_IDS;
    main_deck.shuffle(&mut rand::rng());

    let mut starting_draw_count = 5u32;
    if let Some(card_list) = custom_hand
        && !card_list.is_empty()
    {
        let parsed_custom_hand: CustomHand = CustomHand::try_from(card_list)?;
        starting_draw_count = parsed_custom_hand.0.len() as u32;
        let hand_slice = parsed_custom_hand.0.as_slice();
        pull_ids_to_front(&mut main_deck, hand_slice);
        main_deck.reverse();
    }

    let cards = CardTracker::from_decks(&main_deck, &EXTRA_DECK_IDS)?;
    let duel = core.create_duel(starting_draw_count)?;

    for name in ["constant.lua", "utility.lua"] {
        let script = core
            .script(name)
            .ok_or_else(|| anyhow::anyhow!("{name} is not loaded"))?;
        core.load_script(&duel, &script, name)?;
    }
    for card_id in main_deck {
        duel.add_card(
            Player::Player,
            card_id,
            Player::Player,
            CardLocation::Deck,
            0,
            0,
        );
    }

    for card_id in EXTRA_DECK_IDS {
        duel.add_card(
            Player::Player,
            card_id,
            Player::Player,
            CardLocation::ExtraDeck,
            0,
            10,
        );
    }

    duel.start();

    Ok(DuelSetup { duel, cards })
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
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");

    duel.set_response(response);
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

fn handle_core_messages(messages: &[CoreMessage]) -> bool {
    let mut state = consume_context::<UIState>();
    let mut view = consume_context::<ViewState>();
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");
    let mut tracking = state.cards;
    let playback = try_consume_context::<Signal<CardAnimationState>>();
    let chain = try_consume_context::<Signal<ChainPlayback>>();
    if state.interaction_blocked(playback) {
        return true;
    }

    for message in messages.iter().cloned() {
        tracing::trace!("Received core message: {message:?}");
        if !state.apply_card_message(&mut view, &message, playback) {
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
        match message {
            CoreMessage::Retry => {
                panic!("Received Retry - this shouldn't happen.");
            }
            CoreMessage::NewPhase(message) => {
                state.current_phase.set(Some(message.phase));
            }
            CoreMessage::Idle(actions) => {
                state.pending_input.set(Some(PendingInput::Idle(actions)));
            }
            CoreMessage::SelectPlace(message) => {
                state
                    .pending_input
                    .set(Some(PendingInput::Place(message.zones)));
            }
            CoreMessage::SelectChain(message) => {
                if message.effects.is_empty() {
                    duel.set_response(Response::PassPriority);
                    state.clear_pending_input();
                    view.reset();
                    continue;
                }

                state.pending_input.set(Some(PendingInput::Chain(message)));
            }
            CoreMessage::SelectEffectYN(message) => {
                state
                    .pending_input
                    .set(Some(PendingInput::EffectYesNo(CardData {
                        card_code: message.0.card_code,
                        controller: message.0.controller,
                        location: message.0.location,
                        sequence: message.0.sequence,
                        ..Default::default()
                    })));
            }
            CoreMessage::SelectCard(received_selectables) => {
                state
                    .pending_input
                    .set(Some(PendingInput::Cards(received_selectables)));
            }
            CoreMessage::SelectYesNo(message) => {
                let label = get_card_label(message.card_code).unwrap();
                state.pending_input.set(Some(PendingInput::YesNo(
                    label
                        .optional_strings
                        .get(&message.string_index)
                        .unwrap_or(&String::from("undefined label"))
                        .to_owned(),
                )));
            }
            CoreMessage::SelectPosition(message) => {
                state
                    .pending_input
                    .set(Some(PendingInput::Position(message.positions)));
            }
            CoreMessage::SelectUnselectCard(message) => {
                state
                    .pending_input
                    .set(Some(PendingInput::Unselect(message)));
            }
            CoreMessage::SelectTribute(message) => {
                state.pending_input.set(Some(PendingInput::Tribute {
                    message,
                    selected: Vec::new(),
                }));
            }
            CoreMessage::SelectOption(message) => {
                state
                    .pending_input
                    .set(Some(PendingInput::Options(message)));
            }
            CoreMessage::AnnounceNumber(message) => {
                state
                    .pending_input
                    .set(Some(PendingInput::Numbers(message)));
            }
            CoreMessage::SortCard(message) => {
                state.pending_input.set(Some(PendingInput::Sort(message)));
            }
            _ => {}
        }
    }

    let result = tracking.with_mut(|tracking| {
        let layer = tracking
            .as_mut()
            .expect("Card tracking missing for active duel");
        refresh_data(layer, &duel)?;
        view.reconcile_selection(layer);
        Ok::<_, anyhow::Error>(())
    });
    if let Err(error) = result {
        state.tracking_failed(
            &mut view,
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

fn refresh_data(layer: &mut CardTracker, duel: &Duel) -> anyhow::Result<()> {
    let player = Player::Player;
    for location in [
        CardLocation::Deck,
        CardLocation::Hand,
        CardLocation::MonsterZone,
        CardLocation::SpellTrapZone,
        CardLocation::Graveyard,
        CardLocation::Banishment,
        CardLocation::ExtraDeck,
    ] {
        let core_cards = duel
            .query_cards(player, location)
            .with_context(|| format!("Cannot query {player:?} {location}"))?;
        for card in layer.cards_mut() {
            let CardPlacement::Zone(address) = card.placement else {
                continue;
            };
            if address.controller != player || address.location != location {
                continue;
            }
            let data = core_cards
                .get(address.sequence as usize)
                .and_then(|card| *card)
                .with_context(|| format!("No queried card at {address:?}"))?;
            if location == CardLocation::Graveyard && data.card_code != 0 {
                card.code = data.card_code;
            }
            card.data = Some(data);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/state.rs"]
mod tests;
