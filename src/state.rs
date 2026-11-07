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
use crate::ui::card_layer::{CardAddress, CardId, CardLayer, CardPlacement};
use crate::ui::card_tracking::CardTracking;
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

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedCard {
    pub location: CardLocation,
    pub index: usize,
    card_id: Option<CardId>,
}

impl SelectedCard {
    pub fn matches(self, address: CardAddress) -> bool {
        address.controller == Player::Player
            && self.location == address.location
            && u32::try_from(self.index) == Ok(address.sequence)
    }
}

pub(crate) fn select_card(address: CardAddress) {
    let mut state = consume_context::<UIState>();
    let tracking = consume_context::<Signal<Option<CardTracking>>>();
    let card_id = match &*tracking.peek() {
        Some(CardTracking::Active(layer)) => match layer.card_at(address) {
            Some(card) => Some(card.id),
            None => {
                tracing::error!("Cannot select untracked card at {address:?}");
                return;
            }
        },
        Some(CardTracking::Failed(_)) => None,
        None => {
            tracing::error!("Cannot select a card without an active tracker");
            return;
        }
    };
    let Ok(index) = usize::try_from(address.sequence) else {
        tracing::error!("Card sequence is too large to select: {}", address.sequence);
        return;
    };
    state.selected_card.set(Some(SelectedCard {
        location: address.location,
        index,
        card_id,
    }));
}

fn reconcile_selected_card(state: &mut UIState, tracking: &CardTracking) {
    let Some(selected) = *state.selected_card.peek() else {
        return;
    };
    let Some(id) = selected.card_id else {
        return;
    };
    let CardTracking::Active(layer) = tracking else {
        state.selected_card.set(None);
        return;
    };
    let address = layer.card_by_id(id).and_then(|card| match card.placement {
        CardPlacement::Zone(address) if address.controller == Player::Player => Some(address),
        _ => None,
    });
    match address {
        Some(address) if !selected.matches(address) => match usize::try_from(address.sequence) {
            Ok(index) => state.selected_card.set(Some(SelectedCard {
                location: address.location,
                index,
                card_id: Some(id),
            })),
            Err(error) => {
                tracing::error!("Selected card sequence is invalid: {error}");
                state.selected_card.set(None);
            }
        },
        Some(_) => {}
        None => state.selected_card.set(None),
    }
}

define_ui_state!(UIState {
    main_deck_length: u32,
    current_phase: Option<Phase>,
    extra_deck: Vec<Option<CardData>>,
    hand_contents: Vec<HandCard>,
    selected_card: Option<SelectedCard>,
    normal_summons: Vec<CardData>,
    special_summons: Vec<CardData>,
    monster_sets: Vec<CardData>,
    spell_trap_sets: Vec<CardData>,
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
        process_until_input(|| duel.process(), handle_core_message);
    }
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
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");

    duel.set_response(response);
    state.clear_pending_input();
}

impl UIState {
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

pub fn handle_core_message() -> bool {
    let mut state = consume_context::<UIState>();
    let duel = consume_context::<Signal<Option<Duel>>>();
    let duel = duel().expect("Duel context missing active duel");
    let mut tracking = consume_context::<Signal<Option<CardTracking>>>();

    state.hand_contents.set(duel.get_raw_hand());
    for message in duel.parse_messages() {
        tracking.with_mut(|tracking| {
            let tracking = tracking
                .as_mut()
                .expect("Card tracking missing for active duel");
            tracking.apply(&message);
            reconcile_selected_card(&mut state, tracking);
        });
        match message {
            CoreMessage::Unsupported(_) | CoreMessage::NewTurn | CoreMessage::Win(_) => {}
            CoreMessage::Hint(message) => tracing::trace!("Received hint: {message:?}"),
            CoreMessage::BecomeTarget(message) => {
                tracing::trace!("Received become target: {message:?}")
            }
            CoreMessage::CardHint(message) => tracing::trace!("Received card hint: {message:?}"),
            CoreMessage::CardSelected(message) => {
                tracing::trace!("Received card selected: {message:?}")
            }
            CoreMessage::ConfirmCards(message) => {
                tracing::trace!("Received confirm cards: {message:?}")
            }
            CoreMessage::ConfirmDeckTop(message) => {
                tracing::trace!("Received confirm deck top: {message:?}")
            }
            CoreMessage::ConfirmExtraTop(message) => {
                tracing::trace!("Received confirm extra top: {message:?}")
            }
            CoreMessage::ReverseDeck => tracing::trace!("Received reverse deck"),
            CoreMessage::DeckTop(message) => tracing::trace!("Received deck top: {message:?}"),
            CoreMessage::SwapGraveDeck(message) => {
                tracing::trace!("Received graveyard-deck exchange: {message:?}")
            }
            CoreMessage::ShuffleDeck(message) => {
                tracing::trace!("Received shuffle deck: {message:?}")
            }
            CoreMessage::ShuffleHand(message) => {
                tracing::trace!("Received shuffle hand: {message:?}")
            }
            CoreMessage::ShuffleExtra(message) => {
                tracing::trace!("Received shuffle extra: {message:?}")
            }
            CoreMessage::ShuffleSetCard(message) => {
                tracing::trace!("Received shuffle set card: {message:?}")
            }
            CoreMessage::Move(message) => tracing::trace!("Received move: {message:?}"),
            CoreMessage::Swap(message) => tracing::trace!("Received swap: {message:?}"),
            CoreMessage::RemoveCards(message) => {
                tracing::trace!("Received remove cards: {message:?}")
            }
            CoreMessage::PositionChange(message) => {
                tracing::trace!("Received position change: {message:?}")
            }
            CoreMessage::SetCard(message) => tracing::trace!("Received set card: {message:?}"),
            CoreMessage::Summoning(message) => tracing::trace!("Received summoning: {message:?}"),
            CoreMessage::SpecialSummoning(message) => {
                tracing::trace!("Received special summoning: {message:?}")
            }
            CoreMessage::FlipSummoning(message) => {
                tracing::trace!("Received flip summoning: {message:?}")
            }
            CoreMessage::Summoned => tracing::trace!("Received summoned"),
            CoreMessage::SpecialSummoned => tracing::trace!("Received special summoned"),
            CoreMessage::FlipSummoned => tracing::trace!("Received flip summoned"),
            CoreMessage::Chaining(message) => tracing::trace!("Received chaining: {message:?}"),
            CoreMessage::Chained(message) => tracing::trace!("Received chained: {message:?}"),
            CoreMessage::ChainSolving(message) => {
                tracing::trace!("Received chain solving: {message:?}")
            }
            CoreMessage::ChainSolved(message) => {
                tracing::trace!("Received chain solved: {message:?}")
            }
            CoreMessage::ChainDisabled(message) => {
                tracing::trace!("Received chain disabled: {message:?}")
            }
            CoreMessage::ChainEnd => tracing::trace!("Received chain end"),
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
            CoreMessage::Draw(_) => {}
        }
    }

    tracking.with_mut(|tracking| {
        let tracking = tracking
            .as_mut()
            .expect("Card tracking missing for active duel");
        tracking.reconcile(&duel);
        reconcile_selected_card(&mut state, tracking);
    });

    state
        .main_deck_length
        .set(duel.count_location(Player::Player, CardLocation::Deck));
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
    (state.waiting_on_input)()
}

#[cfg(test)]
#[path = "../tests/unit/state.rs"]
mod tests;
