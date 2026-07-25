use dioxus::prelude::*;
use rand::seq::SliceRandom;
use std::future::pending;

use crate::define_ui_state;
use crate::ocgcore::CardData;
use crate::ocgcore::Duel;
use crate::ocgcore::HandCard;
use crate::ocgcore::OCGCore;
use crate::ocgcore::Response;
use crate::ocgcore::Zone;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::CardOwner;
use crate::ocgcore::constants::DuelStatus;
use crate::ocgcore::messages::AnnounceNumberMessageData;
use crate::ocgcore::messages::CoreMessage;
use crate::ocgcore::messages::SelectCardMessageData;
use crate::ocgcore::messages::SelectOptionMessageData;
use crate::ocgcore::messages::SelectTributeMessageData;
use crate::ocgcore::messages::SelectUnselectMessageData;
use crate::ocgcore::messages::SortCardMessageData;
use crate::settings::CustomHand;
use crate::ui::start_draw_animation;
use crate::utility::EXTRA_DECK_IDS;
use crate::utility::MAIN_DECK_IDS;
use crate::utility::cache_labels;
use crate::utility::cache_scripts;
use crate::utility::get_cached_label;
use crate::utility::get_cached_script;
use crate::utility::get_card_data;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedCard {
    pub location: CardLocation,
    pub index: usize,
}

define_ui_state!(UIState {
    main_deck_length: u32,
    extra_deck: Vec<Option<CardData>>,
    hand_contents: Vec<HandCard>,
    selected_card: Option<SelectedCard>,
    special_summons: Vec<CardData>,
    activatable_effects: Vec<CardData>,
    waiting_on_input: bool,
    monsters: Vec<Option<CardData>>,
    spell_traps: Vec<Option<CardData>>,
    graveyard: Vec<Option<CardData>>,
    banishment: Vec<Option<CardData>>,
    card_prompting_to_activate: Vec<CardData>,
    selectables: Option<SelectCardMessageData>,
    sort_cards_to_select_from: Option<SortCardMessageData>,
    yes_no_question: Option<String>,
    available_zones: Vec<Zone>,
    positions_to_select: Vec<BattlePosition>,
    show_graveyard: bool,
    show_banishment: bool,
    show_extra_deck: bool,
    effects_to_select_from: Vec<CardData>,
    cards_to_select_from: Option<SelectUnselectMessageData>,
    tributes: Option<SelectTributeMessageData>,
    selected_tributes: Vec<u8>,
    options_to_prompt: Option<SelectOptionMessageData>,
    numbers_to_select_from: Option<AnnounceNumberMessageData>,
});

pub fn run_game_loop() {
    let state = consume_context::<UIState>();
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");

    if !(state.waiting_on_input)() {
        loop {
            match duel.process() {
                DuelStatus::Awaiting | DuelStatus::Continue => {
                    handle_core_message();
                    break;
                }
                DuelStatus::End => break,
            }
        }
    }
}

pub fn handle_right_click(evt: MouseEvent) {
    let mut state = consume_context::<UIState>();

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

    if (state.show_graveyard)() {
        state.show_graveyard.set(false);
    }

    if (state.show_extra_deck)() {
        state.show_extra_deck.set(false);
    }

    if (state.show_banishment)() {
        state.show_banishment.set(false);
    }

    if (state.tributes)().is_some_and(|message| message.is_cancelable) {
        send_response(Response::PassPriority);
    }

    evt.prevent_default();
}

pub async fn cache_cards() {
    let all_cards = MAIN_DECK_IDS
        .into_iter()
        .chain(EXTRA_DECK_IDS)
        .collect::<Vec<_>>();

    cache_scripts(&all_cards).await;
    cache_labels(&all_cards).await;
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

pub async fn load_core() -> anyhow::Result<OCGCore> {
    OCGCore::load(get_card_data, get_cached_script, |text| {
        tracing::info!(text)
    })
    .await
}

pub async fn load_duel(
    core_resource: Resource<anyhow::Result<OCGCore>>,
    custom_hand: Option<String>,
) -> anyhow::Result<Duel> {
    let core = core_resource
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok().cloned());

    let core = match core {
        Some(core) => core,
        None => return pending().await,
    };

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

    let duel = core.create_duel(starting_draw_count)?;

    core.load_script(
        &duel,
        get_cached_script("constant.lua").unwrap(),
        "constant.lua",
    );
    core.load_script(
        &duel,
        get_cached_script("utility.lua").unwrap(),
        "utility.lua",
    );

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

pub fn send_response(response: Response) {
    let mut state = consume_context::<UIState>();
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");

    duel.set_response(response);
    state.clear_pending_input();
}

impl UIState {
    pub fn clear_pending_input(&mut self) {
        self.selected_card.set(None);
        self.special_summons.set(Vec::new());
        self.activatable_effects.set(Vec::new());
        self.waiting_on_input.set(false);
        self.card_prompting_to_activate.set(Vec::new());
        self.selectables.set(None);
        self.sort_cards_to_select_from.set(None);
        self.yes_no_question.set(None);
        self.available_zones.set(Vec::new());
        self.positions_to_select.set(Vec::new());
        self.show_graveyard.set(false);
        self.show_banishment.set(false);
        self.show_extra_deck.set(false);
        self.effects_to_select_from.set(Vec::new());
        self.cards_to_select_from.set(None);
        self.tributes.set(None);
        self.selected_tributes.set(Vec::new());
        self.options_to_prompt.set(None);
        self.numbers_to_select_from.set(None);
    }
}

pub fn handle_core_message() {
    let mut state = consume_context::<UIState>();
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");

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
        CoreMessage::SelectPlace(message) => {
            state.available_zones.set(message.zones);
            state.waiting_on_input.set(true);
        }
        CoreMessage::SelectChain(message) => {
            if message.effects.is_empty() {
                send_response(Response::PassPriority);
                return;
            }

            state.hand_contents.with_mut(|hand| {
                for effect in &message.effects {
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
            let label = get_cached_label(message.card_code).unwrap();
            state.yes_no_question.set(Some(
                label
                    .optional_strings
                    .get(&message.string_index)
                    .unwrap_or(&String::from("undefined label"))
                    .to_owned(),
            ));
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
        CoreMessage::Draw(message) => {
            let hand_size = (state.hand_contents)().len();
            if start_draw_animation(message.cards, hand_size, state) {
                state.waiting_on_input.set(true);
            }
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
    state
        .banishment
        .set(duel.get_cards(CardLocation::Banishment));
}
