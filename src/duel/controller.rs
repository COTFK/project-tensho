use anyhow::Context;
use rand::seq::SliceRandom;

use super::cards::{CardPlacement, CardTracker};
use super::input::PendingInput;
use crate::ocgcore::constants::{CardLocation, Phase, Player};
use crate::ocgcore::messages::CoreMessage;
use crate::ocgcore::{CardData, Duel, OCGCore, Response, log_core_message};
use crate::settings::CustomHand;
use crate::utility::{
    ARCHETYPE_SCRIPT, EXTRA_DECK_IDS, MAIN_DECK_IDS, Scripts, cache_scripts, fetch_card_data,
    get_card_data, get_card_label, store_card_data,
};
use ocgcore_ffi::enums::OCG_DuelStatus;

#[derive(Clone)]
pub(crate) struct DuelController {
    duel: Duel,
}

#[derive(Clone)]
pub(crate) struct DuelSetup {
    pub controller: DuelController,
    pub cards: CardTracker,
}

impl DuelController {
    pub fn new(duel: Duel) -> Self {
        Self { duel }
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.duel.ptr_eq(&other.duel)
    }

    pub fn process(&self) -> OCG_DuelStatus {
        self.duel.process()
    }

    pub fn parse_messages(&self) -> Vec<CoreMessage> {
        self.duel.parse_messages()
    }

    pub fn send_response(&self, response: Response) {
        self.duel.set_response(response);
    }

    // Returns whether the request was answered automatically.
    pub fn handle_message(
        &self,
        message: CoreMessage,
        pending_input: &mut Option<PendingInput>,
        current_phase: &mut Option<Phase>,
    ) -> bool {
        match message {
            CoreMessage::Retry => {
                panic!("Received Retry - this shouldn't happen.");
            }
            CoreMessage::NewPhase(message) => {
                *current_phase = Some(message.phase);
            }
            CoreMessage::Idle(actions) => {
                *pending_input = Some(PendingInput::Idle(actions));
            }
            CoreMessage::SelectPlace(message) => {
                *pending_input = Some(PendingInput::Place(message.zones));
            }
            CoreMessage::SelectChain(message) => {
                if message.effects.is_empty() {
                    self.send_response(Response::PassPriority);
                    *pending_input = None;
                    return true;
                }
                *pending_input = Some(PendingInput::Chain(message));
            }
            CoreMessage::SelectEffectYN(message) => {
                *pending_input = Some(PendingInput::EffectYesNo(CardData {
                    card_code: message.0.card_code,
                    controller: message.0.controller,
                    location: message.0.location,
                    sequence: message.0.sequence,
                    ..Default::default()
                }));
            }
            CoreMessage::SelectCard(message) => {
                *pending_input = Some(PendingInput::Cards(message));
            }
            CoreMessage::SelectYesNo(message) => {
                let label = get_card_label(message.card_code).unwrap();
                *pending_input = Some(PendingInput::YesNo(
                    label
                        .optional_strings
                        .get(&message.string_index)
                        .unwrap_or(&String::from("undefined label"))
                        .to_owned(),
                ));
            }
            CoreMessage::SelectPosition(message) => {
                *pending_input = Some(PendingInput::Position(message.positions));
            }
            CoreMessage::SelectUnselectCard(message) => {
                *pending_input = Some(PendingInput::Unselect(message));
            }
            CoreMessage::SelectTribute(message) => {
                *pending_input = Some(PendingInput::Tribute {
                    message,
                    selected: Vec::new(),
                });
            }
            CoreMessage::SelectOption(message) => {
                *pending_input = Some(PendingInput::Options(message));
            }
            CoreMessage::AnnounceNumber(message) => {
                *pending_input = Some(PendingInput::Numbers(message));
            }
            CoreMessage::SortCard(message) => {
                *pending_input = Some(PendingInput::Sort(message));
            }
            _ => {}
        }
        false
    }

    pub fn refresh_data(&self, cards: &mut CardTracker) -> anyhow::Result<()> {
        refresh_data(cards, &self.duel)
    }
}

pub(crate) fn process_until_input(
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

pub(crate) async fn cache_cards() -> anyhow::Result<Scripts> {
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

pub(crate) async fn load_core(scripts: Scripts) -> anyhow::Result<OCGCore> {
    OCGCore::load(
        get_card_data,
        Box::new(move |name: &str| scripts.get(name).cloned()),
        log_core_message,
    )
    .await
}

pub(crate) async fn load_duel(
    core: OCGCore,
    custom_hand: Option<String>,
) -> anyhow::Result<DuelSetup> {
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
        pull_ids_to_front(&mut main_deck, parsed_custom_hand.0.as_slice());
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
    Ok(DuelSetup {
        controller: DuelController::new(duel),
        cards,
    })
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
#[path = "../../tests/unit/duel/controller.rs"]
mod tests;
