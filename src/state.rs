use dioxus::prelude::*;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::future::pending;

use crate::ocgcore::ActiveCard;
use crate::ocgcore::CoreMessage;
use crate::ocgcore::Duel;
use crate::ocgcore::DuelStatus;
use crate::ocgcore::OCGCore;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::CardOwner;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_labels;
use crate::utility::cache_scripts;
use crate::utility::get_cached_label;
use crate::utility::get_cached_script;

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedCard {
    pub location: CardLocation,
    pub index: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuelState {
    pub duel: Duel,
    pub main_deck_length: Signal<u32>,
    pub extra_deck_length: Signal<u32>,
    pub hand_contents: Signal<Vec<Option<ActiveCard>>>,
    pub selected_card: Signal<Option<SelectedCard>>,
    pub normal_summons: Signal<HashMap<u8, u16>>,
    pub activatable_effects: Signal<HashMap<u16, ActiveCard>>,
    pub waiting_on_input: Signal<bool>,
    pub monsters: Signal<Vec<Option<ActiveCard>>>,
    pub spell_traps: Signal<Vec<Option<ActiveCard>>>,
    pub graveyard: Signal<Vec<Option<ActiveCard>>>,
    pub card_prompting_to_activate: Signal<Vec<ActiveCard>>,
    pub selectables: Signal<Vec<ActiveCard>>,
    pub yes_no_question: Signal<Option<String>>,
    pub available_zones: Signal<Vec<(CardLocation, u8)>>,
    pub positions_to_select: Signal<Vec<BattlePosition>>,
    pub show_graveyard: Signal<bool>,
    pub effects_to_select_from: Signal<Vec<(u16, ActiveCard)>>,
}

impl DuelState {
    pub fn new(duel: Duel) -> Self {
        Self {
            duel,
            main_deck_length: use_signal(|| 0),
            extra_deck_length: use_signal(|| 0),
            hand_contents: use_signal(Vec::new),
            selected_card: use_signal(|| None),
            normal_summons: use_signal(HashMap::new),
            activatable_effects: use_signal(HashMap::new),
            waiting_on_input: use_signal(|| false),
            monsters: use_signal(Vec::new),
            spell_traps: use_signal(Vec::new),
            graveyard: use_signal(Vec::new),
            card_prompting_to_activate: use_signal(Vec::new),
            selectables: use_signal(Vec::new),
            yes_no_question: use_signal(|| None),
            available_zones: use_signal(Vec::new),
            positions_to_select: use_signal(Vec::new),
            show_graveyard: use_signal(|| false),
            effects_to_select_from: use_signal(Vec::new),
        }
    }
}

pub fn run_game_loop() {
    let state = use_context::<DuelState>();

    if !(state.waiting_on_input)() {
        loop {
            match state.duel.process() {
                DuelStatus::Awaiting => {
                    handle_core_message();
                    break;
                }
                DuelStatus::Continue => continue,
                DuelStatus::End => break,
            }
        }
    }
}

pub fn handle_right_click(evt: MouseEvent) {
    let mut state = use_context::<DuelState>();

    // Allow to decline chains & activations
    if !(state.card_prompting_to_activate)().is_empty() {
        if (state.card_prompting_to_activate)()
            .iter()
            .any(|card| card.chain_option.is_some())
        {
            send_user_response(UserResponse::PassPriority)
        } else {
            send_user_response(UserResponse::No);
        }

        state.card_prompting_to_activate.with_mut(|v| v.clear());
    }

    // Decline Yes/No questions
    if (state.yes_no_question)().is_some() {
        send_user_response(UserResponse::No);
        state.yes_no_question.set(None);
    }

    if (state.show_graveyard)() {
        state.show_graveyard.set(false);
    }

    evt.prevent_default();
}

pub fn handle_left_click() {
    let mut state = use_context::<DuelState>();
    if (state.selected_card)().is_some() {
        state.selected_card.set(None);
    }
}

pub async fn cache_dependencies() -> anyhow::Result<OCGCore> {
    let all_cards = MAIN_DECK_IDS
        .into_iter()
        .chain(EXTRA_DECK_IDS)
        .collect::<Vec<_>>();

    cache_scripts(&all_cards).await;
    cache_labels(&all_cards).await;

    OCGCore::load().await
}

pub async fn load_duel(cache_resource: Resource<anyhow::Result<OCGCore>>) -> anyhow::Result<Duel> {
    let core = {
        let cache_state = cache_resource.read();
        match &*cache_state {
            Some(Ok(core)) => Some(core.clone()),
            Some(Err(err)) => return Err(anyhow::anyhow!("Core initialization failed: {err:#}")),
            None => None,
        }
    };

    let core = match core {
        Some(core) => core,
        None => pending().await,
    };

    let duel = core.create_duel()?;

    duel.load_script(get_cached_script("constant.lua").unwrap(), "constant.lua");
    duel.load_script(get_cached_script("utility.lua").unwrap(), "utility.lua");

    let mut main_deck = MAIN_DECK_IDS;
    main_deck.shuffle(&mut rand::rng());

    for card_id in main_deck {
        duel.add_card(
            CardOwner::Player,
            card_id,
            CardController::Player,
            CardLocation::Deck,
            0,
            0,
        );
    }

    for card_id in EXTRA_DECK_IDS {
        duel.add_card(
            CardOwner::Player,
            card_id,
            CardController::Player,
            CardLocation::ExtraDeck,
            0,
            0,
        );
    }

    duel.start();
    debug!("Duel started successfully.");

    Ok(duel)
}

pub fn send_user_response(response: UserResponse) {
    let mut state = use_context::<DuelState>();

    state.duel.set_response(response);

    // Clean up state and wait for new data
    state.hand_contents.clear();
    state.selected_card.set(None);
    state.normal_summons.clear();
    state.activatable_effects.clear();
    state.waiting_on_input.set(false);
    state.monsters.clear();
    state.spell_traps.clear();
    state.card_prompting_to_activate.clear();
    state.selectables.clear();
    state.yes_no_question.set(None);
    state.available_zones.clear();
    state.positions_to_select.clear();
    state.show_graveyard.set(false);
    state.effects_to_select_from.clear();
}

pub fn handle_core_message() {
    let mut state = use_context::<DuelState>();

    match state.duel.parse_messages() {
        CoreMessage::Retry => {
            warn!("Received Retry - this shouldn't happen.");
        }
        CoreMessage::Idle(actions) => {
            state.normal_summons.set(actions.get_normal_summons());
            state
                .activatable_effects
                .set(actions.get_activatable_effects());
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectPlace(zones) => {
            state.available_zones.set(zones);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectChain(effects) => {
            if effects.is_empty() {
                send_user_response(UserResponse::PassPriority);
            } else {
                state.card_prompting_to_activate.set(effects);
                state.waiting_on_input.set(true);
            }
        }
        CoreMessage::SelectEffectYN(effect) => {
            state
                .card_prompting_to_activate
                .with_mut(|v| v.push(effect));
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectCard(received_selectables) => {
            debug!("selecting from {:?}", received_selectables);
            state.selectables.set(received_selectables);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectYesNo {
            player: _,
            card_code,
            string_index,
        } => {
            debug!("got {card_code}, {string_index}");

            let label = get_cached_label(card_code).unwrap();
            state.yes_no_question.set(Some(
                label
                    .optional_strings
                    .get(&string_index)
                    .unwrap_or(&String::from("undefined label"))
                    .to_owned(),
            ));
        }
        CoreMessage::SelectPosition(positions) => {
            state.positions_to_select.set(positions);
            state.waiting_on_input.set(true);
        }
    }

    state.main_deck_length.set(
        state
            .duel
            .count_location(CardOwner::Player, CardLocation::Deck),
    );
    state.extra_deck_length.set(
        state
            .duel
            .count_location(CardOwner::Player, CardLocation::ExtraDeck),
    );

    state
        .monsters
        .set(state.duel.get_cards(CardLocation::MonsterZone));
    state
        .spell_traps
        .set(state.duel.get_cards(CardLocation::SpellTrapZone));
    state
        .hand_contents
        .set(state.duel.get_cards(CardLocation::Hand));
    state
        .graveyard
        .set(state.duel.get_cards(CardLocation::Graveyard));
}
