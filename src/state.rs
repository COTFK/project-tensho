use anyhow::Context;
use dioxus::prelude::*;
use rand::seq::SliceRandom;
use std::future::pending;

use crate::define_ui_state;
use crate::ocgcore::CardData;
use crate::ocgcore::Duel;
use crate::ocgcore::OCGCore;
use crate::ocgcore::Response;
use crate::ocgcore::Zone;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::Phase;
use crate::ocgcore::constants::Player;
use crate::ocgcore::log_core_message;
use crate::ocgcore::messages::AnnounceNumberMessageData;
use crate::ocgcore::messages::CoreMessage;
use crate::ocgcore::messages::SelectCardMessageData;
use crate::ocgcore::messages::SelectOptionMessageData;
use crate::ocgcore::messages::SelectTributeMessageData;
use crate::ocgcore::messages::SelectUnselectMessageData;
use crate::ocgcore::messages::SortCardMessageData;
use crate::settings::CustomHand;
use crate::ui::card_animation::{CardAnimationState, plan_animation};
use crate::ui::card_layer::{CardAddress, CardId, CardLayer, CardPlacement};
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
    let mut state = consume_context::<UIState>();
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
    state.selected_card.set(Some(card_id));
}

fn reconcile_selected_card(state: &mut UIState, layer: &CardLayer) {
    let Some(id) = *state.selected_card.peek() else {
        return;
    };
    if !layer.card_by_id(id).is_some_and(|card| {
        matches!(card.placement, CardPlacement::Zone(address) if address.controller == Player::Player)
    }) {
        state.selected_card.set(None);
    }
}

define_ui_state!(UIState {
    cards: Option<CardLayer>,
    current_phase: Option<Phase>,
    selected_card: Option<CardId>,
    normal_summons: Vec<CardData>,
    special_summons: Vec<CardData>,
    monster_sets: Vec<CardData>,
    spell_trap_sets: Vec<CardData>,
    activatable_effects: Vec<CardData>,
    waiting_on_input: bool,
    duel_finished: bool,
    restarting: bool,
    failure: Option<String>,
    card_prompting_to_activate: Vec<CardData>,
    selectables: Option<SelectCardMessageData>,
    sort_cards_to_select_from: Option<SortCardMessageData>,
    yes_no_question: Option<String>,
    available_zones: Vec<Zone>,
    positions_to_select: Vec<BattlePosition>,
    open_pile: Option<CardLocation>,
    effects_to_select_from: Vec<CardData>,
    cards_to_select_from: Option<SelectUnselectMessageData>,
    tributes: Option<SelectTributeMessageData>,
    selected_tributes: Vec<u8>,
    options_to_prompt: Option<SelectOptionMessageData>,
    numbers_to_select_from: Option<AnnounceNumberMessageData>,
});

pub fn run_game_loop() {
    let state = consume_context::<UIState>();
    let playback = try_consume_context::<Signal<CardAnimationState>>();
    let tracking = state.cards;
    if state.interaction_blocked(playback) || (state.duel_finished)() || (state.waiting_on_input)()
    {
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
            if state.interaction_blocked(playback) {
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
            waiting
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
    let mut state = consume_context::<UIState>();
    if state.interaction_blocked(try_consume_context::<Signal<CardAnimationState>>()) {
        return;
    }

    // Allow to decline chains & activations
    if !(state.card_prompting_to_activate)().is_empty() {
        if (state.card_prompting_to_activate)()
            .iter()
            .any(|card| card.action_index.is_some())
        {
            send_response(Response::PassPriority)
        } else {
            send_response(Response::No);
        }
    }

    // Decline Yes/No questions
    if (state.yes_no_question)().is_some() {
        send_response(Response::No);
    }

    state.open_pile.set(None);

    if (state.tributes)().is_some_and(|message| message.is_cancelable) {
        send_response(Response::PassPriority);
    }
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
    pub cards: CardLayer,
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

    let cards = CardLayer::from_decks(&main_deck, &EXTRA_DECK_IDS)?;
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
    if !*state.waiting_on_input.peek() {
        tracing::warn!("Ignoring response without a pending prompt");
        return;
    }
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");

    duel.set_response(response);
    state.clear_pending_input();
}

impl UIState {
    pub(crate) fn interaction_blocked(&self, playback: Option<Signal<CardAnimationState>>) -> bool {
        (self.restarting)()
            || (self.failure)().is_some()
            || playback.is_some_and(|playback| playback().is_active())
    }

    pub(crate) fn fail(&mut self, error: String, playback: Option<Signal<CardAnimationState>>) {
        if self.failure.peek().is_none() {
            if let Some(mut playback) = playback {
                playback.write().cancel();
            }
            self.failure.set(Some(error));
        }
    }

    fn tracking_failed(&mut self, error: String, playback: Option<Signal<CardAnimationState>>) {
        tracing::error!("{error}");
        self.cards.set(None);
        self.selected_card.set(None);
        self.fail(error, playback);
    }

    fn apply_card_message(
        &mut self,
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
            reconcile_selected_card(self, layer);
            Ok::<_, anyhow::Error>(())
        });
        if let Err(error) = result {
            self.tracking_failed(
                format!("Card tracking failed for {message:?}: {error:#}"),
                playback,
            );
            return false;
        }
        true
    }

    pub(crate) fn is_selected(&self, address: CardAddress) -> bool {
        let selected = (self.selected_card)();
        match &*(self.cards).read() {
            Some(cards) => cards
                .card_at(address)
                .is_some_and(|card| Some(card.id) == selected),
            _ => false,
        }
    }

    pub(crate) fn zone_data(&self, location: CardLocation) -> Option<Vec<Option<CardData>>> {
        match &*self.cards.read() {
            Some(cards) => cards.zone_data(location),
            _ => None,
        }
    }

    pub fn clear_pending_input(&mut self) {
        self.selected_card.set(None);
        self.normal_summons.set(Vec::new());
        self.special_summons.set(Vec::new());
        self.monster_sets.set(Vec::new());
        self.spell_trap_sets.set(Vec::new());
        self.activatable_effects.set(Vec::new());
        self.waiting_on_input.set(false);
        self.card_prompting_to_activate.set(Vec::new());
        self.selectables.set(None);
        self.sort_cards_to_select_from.set(None);
        self.yes_no_question.set(None);
        self.available_zones.set(Vec::new());
        self.positions_to_select.set(Vec::new());
        self.open_pile.set(None);
        self.effects_to_select_from.set(Vec::new());
        self.cards_to_select_from.set(None);
        self.tributes.set(None);
        self.selected_tributes.set(Vec::new());
        self.options_to_prompt.set(None);
        self.numbers_to_select_from.set(None);
    }
}

fn handle_core_messages(messages: &[CoreMessage]) -> bool {
    let mut state = consume_context::<UIState>();
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");
    let mut tracking = state.cards;
    let playback = try_consume_context::<Signal<CardAnimationState>>();
    if state.interaction_blocked(playback) {
        return true;
    }

    for message in messages.iter().cloned() {
        tracing::trace!("Received core message: {message:?}");
        if !state.apply_card_message(&message, playback) {
            return true;
        }
        match message {
            CoreMessage::Retry => {
                panic!("Received Retry - this shouldn't happen.");
            }
            CoreMessage::NewPhase(message) => {
                state.current_phase.set(Some(message.phase));
            }
            CoreMessage::Idle(actions) => {
                state.normal_summons.set(actions.normal_summons);
                state.special_summons.set(actions.special_summons);
                state.monster_sets.set(actions.monster_sets);
                state.spell_trap_sets.set(actions.spell_trap_sets);
                state.activatable_effects.set(actions.activatable_effects);
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectPlace(message) => {
                state.available_zones.set(message.zones);
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectChain(message) => {
                if message.effects.is_empty() {
                    duel.set_response(Response::PassPriority);
                    state.clear_pending_input();
                    continue;
                }

                state.card_prompting_to_activate.set(message.effects);
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectEffectYN(message) => {
                state.card_prompting_to_activate.with_mut(|v| {
                    v.push(CardData {
                        card_code: message.0.card_code,
                        controller: message.0.controller,
                        location: message.0.location,
                        sequence: message.0.sequence,
                        ..Default::default()
                    })
                });
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectCard(received_selectables) => {
                state.selectables.set(Some(received_selectables));
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectYesNo(message) => {
                let label = get_card_label(message.card_code).unwrap();
                state.yes_no_question.set(Some(
                    label
                        .optional_strings
                        .get(&message.string_index)
                        .unwrap_or(&String::from("undefined label"))
                        .to_owned(),
                ));
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectPosition(message) => {
                state.positions_to_select.set(message.positions);
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectUnselectCard(message) => {
                state.cards_to_select_from.set(Some(message));
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectTribute(message) => {
                state.tributes.set(Some(message));
                state.waiting_on_input.set(true);
            }
            CoreMessage::SelectOption(message) => {
                state.options_to_prompt.set(Some(message));
                state.waiting_on_input.set(true);
            }
            CoreMessage::AnnounceNumber(message) => {
                state.numbers_to_select_from.set(Some(message));
                state.waiting_on_input.set(true);
            }
            CoreMessage::SortCard(message) => {
                state.sort_cards_to_select_from.set(Some(message));
                state.waiting_on_input.set(true);
            }
            _ => {}
        }
    }

    let result = tracking.with_mut(|tracking| {
        let layer = tracking
            .as_mut()
            .expect("Card tracking missing for active duel");
        refresh_data(layer, &duel)?;
        reconcile_selected_card(&mut state, layer);
        Ok::<_, anyhow::Error>(())
    });
    if let Err(error) = result {
        state.tracking_failed(format!("Cannot refresh card data: {error:#}"), playback);
        return true;
    }

    (state.waiting_on_input)()
}

fn refresh_data(layer: &mut CardLayer, duel: &Duel) -> anyhow::Result<()> {
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
