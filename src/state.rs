use dioxus::prelude::*;
use rand::seq::SliceRandom;
use std::future::pending;

use crate::ocgcore::CardData;
use crate::ocgcore::CoreMessage;
use crate::ocgcore::Duel;
use crate::ocgcore::DuelStatus;
use crate::ocgcore::HandCard;
use crate::ocgcore::OCGCore;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::CardOwner;
use crate::ocgcore::messages::SelectTributeMessageData;
use crate::ocgcore::messages::SelectUnselectMessageData;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_labels;
use crate::utility::cache_scripts;
use crate::utility::get_cached_label;
use crate::utility::get_cached_script;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedCard {
    pub location: CardLocation,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuelState {
    pub duel: Signal<Duel>,
    pub main_deck_length: Signal<u32>,
    pub extra_deck: Signal<Vec<Option<CardData>>>,
    pub hand_contents: Signal<Vec<HandCard>>,
    pub selected_card: Signal<Option<SelectedCard>>,
    pub special_summons: Signal<Vec<CardData>>,
    pub activatable_effects: Signal<Vec<CardData>>,
    pub waiting_on_input: Signal<bool>,
    pub monsters: Signal<Vec<Option<CardData>>>,
    pub spell_traps: Signal<Vec<Option<CardData>>>,
    pub graveyard: Signal<Vec<Option<CardData>>>,
    pub card_prompting_to_activate: Signal<Vec<CardData>>,
    pub selectables: Signal<Vec<CardData>>,
    pub yes_no_question: Signal<Option<String>>,
    pub available_zones: Signal<Vec<(CardLocation, u8)>>,
    pub positions_to_select: Signal<Vec<BattlePosition>>,
    pub show_graveyard: Signal<bool>,
    pub show_extra_deck: Signal<bool>,
    pub effects_to_select_from: Signal<Vec<CardData>>,
    pub cards_to_select_from: Signal<Option<SelectUnselectMessageData>>,
    pub tributes: Signal<Option<SelectTributeMessageData>>,
    pub selected_tributes: Signal<Vec<u8>>,
}

impl DuelState {
    pub fn new(duel: Duel) -> Self {
        Self {
            duel: use_signal(move || duel),
            main_deck_length: use_signal(|| 0),
            extra_deck: use_signal(Vec::new),
            hand_contents: use_signal(Vec::new),
            selected_card: use_signal(|| None),
            special_summons: use_signal(Vec::new),
            activatable_effects: use_signal(Vec::new),
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
            show_extra_deck: use_signal(|| false),
            effects_to_select_from: use_signal(Vec::new),
            cards_to_select_from: use_signal(|| None),
            tributes: use_signal(|| None),
            selected_tributes: use_signal(Vec::new),
        }
    }

    pub fn reset(&mut self, duel: Duel) {
        if self.duel.read().clone() == duel {
            return;
        }

        (self.duel)().destroy();
        self.duel.set(duel);

        self.main_deck_length.set(0);
        self.extra_deck.clear();
        self.hand_contents.clear();
        self.selected_card.set(None);
        self.special_summons.clear();
        self.activatable_effects.clear();
        self.waiting_on_input.set(false);
        self.monsters.clear();
        self.spell_traps.clear();
        self.graveyard.clear();
        self.card_prompting_to_activate.clear();
        self.selectables.clear();
        self.yes_no_question.set(None);
        self.available_zones.clear();
        self.positions_to_select.clear();
        self.show_graveyard.set(false);
        self.cards_to_select_from.set(None);
        self.effects_to_select_from.clear();
        self.tributes.set(None);
        self.selected_tributes.clear();
    }
}

pub fn run_game_loop() {
    let state = use_context::<DuelState>();

    if !(state.waiting_on_input)() {
        loop {
            match (state.duel)().process() {
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
            .any(|card| card.action_index.is_some())
        {
            send_user_response(UserResponse::PassPriority)
        } else {
            send_user_response(UserResponse::No);
        }
    }

    // Decline Yes/No questions
    if (state.yes_no_question)().is_some() {
        send_user_response(UserResponse::No);
    }

    if (state.show_graveyard)() {
        state.show_graveyard.set(false);
    }

    if (state.tributes)().is_some_and(|message| message.is_cancelable) {
        send_user_response(UserResponse::PassPriority);
    }

    evt.prevent_default();
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
            10,
        );
    }

    duel.start();

    Ok(duel)
}

pub fn send_user_response(response: UserResponse) {
    let mut state = use_context::<DuelState>();

    state.duel.read().set_response(response);

    // Clean up state and wait for new data
    state.hand_contents.clear();
    state.selected_card.set(None);
    state.special_summons.clear();
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
    state.cards_to_select_from.set(None);
    state.effects_to_select_from.clear();
    state.tributes.set(None);
    state.selected_tributes.clear();
}

pub fn handle_core_message() {
    let mut state = use_context::<DuelState>();
    let duel = (state.duel)();

    state.hand_contents.set(duel.get_raw_hand());

    match duel.parse_messages() {
        CoreMessage::Retry => {
            panic!("Received Retry - this shouldn't happen.");
        }
        CoreMessage::Idle(actions) => {
            let normal_summons = actions.normal_summons;
            let special_summons = actions.special_summons;
            let activatable_effects = actions.activatable_effects;
            let monster_sets = actions.monster_sets;
            let spell_trap_sets = actions.spell_trap_sets;

            if !normal_summons.is_empty()
                || !activatable_effects.is_empty()
                || !spell_trap_sets.is_empty()
            {
                state.hand_contents.with_mut(|hand| {
                    for summon in &normal_summons {
                        if summon.location != CardLocation::Hand {
                            continue;
                        }
                        if let Some(hc) =
                            hand.iter_mut().find(|hc| hc.index as u8 == summon.sequence)
                        {
                            hc.normal_summon_index = summon.action_index;
                        }
                    }

                    for set in &monster_sets {
                        if set.location != CardLocation::Hand {
                            continue;
                        }
                        if let Some(hc) = hand.iter_mut().find(|hc| hc.index as u8 == set.sequence)
                        {
                            hc.monster_set_index = set.action_index;
                        }
                    }

                    for set in &spell_trap_sets {
                        if set.location != CardLocation::Hand {
                            continue;
                        }
                        if let Some(hc) = hand.iter_mut().find(|hc| hc.index as u8 == set.sequence)
                        {
                            hc.spell_trap_set_index = set.action_index;
                        }
                    }

                    for effect in &activatable_effects {
                        if effect.location != CardLocation::Hand {
                            continue;
                        }
                        if let Some(hc) =
                            hand.iter_mut().find(|hc| hc.index as u8 == effect.sequence)
                        {
                            hc.activate_index = effect.action_index;
                            hc.is_activatable_or_chainable = true;
                        }
                    }
                });
            }

            state.special_summons.set(special_summons.to_owned());
            state
                .activatable_effects
                .set(activatable_effects.to_owned());
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectPlace(zones) => {
            state.available_zones.set(zones);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectChain(effects) => {
            if effects.is_empty() {
                send_user_response(UserResponse::PassPriority);
                return;
            }

            state.hand_contents.with_mut(|hand| {
                for effect in &effects {
                    if effect.location != CardLocation::Hand {
                        continue;
                    }
                    if let Some(hand_card) =
                        hand.iter_mut().find(|hc| hc.index as u8 == effect.sequence)
                    {
                        hand_card.chain_index = effect.action_index;
                        hand_card.is_activatable_or_chainable = true;
                    }
                }
            });

            state.card_prompting_to_activate.set(effects);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectEffectYN(effect) => {
            state
                .card_prompting_to_activate
                .with_mut(|v| v.push(effect));
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectCard(received_selectables) => {
            state.selectables.set(received_selectables.cards);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectYesNo {
            player: _,
            card_code,
            string_index,
        } => {
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
        CoreMessage::SelectUnselectCard(message) => {
            state.cards_to_select_from.set(Some(message));
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectTribute(message) => {
            state.tributes.set(Some(message));
            state.waiting_on_input.set(true);
        }
    }

    state
        .main_deck_length
        .set(duel.count_location(CardOwner::Player, CardLocation::Deck));
    state
        .extra_deck
        .set(duel.get_cards(CardLocation::ExtraDeck));
    state
        .monsters
        .set(duel.get_cards(CardLocation::MonsterZone));
    state
        .spell_traps
        .set(duel.get_cards(CardLocation::SpellTrapZone));
    state.graveyard.set(duel.get_cards(CardLocation::Graveyard));
}
