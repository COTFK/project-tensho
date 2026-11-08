use anyhow::Context;
use rand::seq::SliceRandom;

use super::cards::{CardInstanceId, CardPlacement, CardTracker};
use super::input::PendingInput;
use crate::ocgcore::constants::{CardLocation, Phase, Player};
use crate::ocgcore::messages::CoreMessage;
use crate::ocgcore::{Duel, DuelCardData, OCGCore, Response, log_core_message};
use crate::settings::CustomHand;
use crate::utility::{
    ARCHETYPE_SCRIPT, EXTRA_DECK_CODES, MAIN_DECK_CODES, Scripts, cache_scripts,
    fetch_card_definitions, get_card_text, read_card_definition, store_card_definitions,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChainLink {
    pub number: u32,
    pub card: Option<CardInstanceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChainEvent {
    Added(ChainLink),
    Solved(u32),
    End,
}

pub(crate) fn resolve_chain_event(
    message: &CoreMessage,
    cards: &CardTracker,
) -> Option<ChainEvent> {
    match message {
        CoreMessage::Chaining(message) => {
            let card = match cards.card_at_location(message.location) {
                Ok(Some(card)) => Some(card.id),
                Ok(None) => {
                    tracing::warn!(
                        "No tracked card for chain link {} at {:?}",
                        message.chain_link,
                        message.location
                    );
                    None
                }
                Err(error) => {
                    tracing::warn!(
                        "Cannot locate card for chain link {}: {error:#}",
                        message.chain_link
                    );
                    None
                }
            };
            Some(ChainEvent::Added(ChainLink {
                number: message.chain_link,
                card,
            }))
        }
        CoreMessage::ChainSolved(message) => {
            Some(ChainEvent::Solved(u32::from(message.chain_link)))
        }
        CoreMessage::ChainEnd => Some(ChainEvent::End),
        _ => None,
    }
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
                *pending_input = Some(PendingInput::ActionSelection(actions));
            }
            CoreMessage::SelectPlace(message) => {
                *pending_input = Some(PendingInput::FieldPlacement(message.zones));
            }
            CoreMessage::SelectChain(message) => {
                if message.effects.is_empty() {
                    self.send_response(Response::PassPriority);
                    *pending_input = None;
                    return true;
                }
                *pending_input = Some(PendingInput::ChainResponse(message));
            }
            CoreMessage::SelectEffectYN(message) => {
                *pending_input = Some(PendingInput::EffectConfirmation(DuelCardData {
                    card_code: message.0.card_code,
                    controller: message.0.controller,
                    location: message.0.location,
                    sequence: message.0.sequence,
                    ..Default::default()
                }));
            }
            CoreMessage::SelectCard(message) => {
                *pending_input = Some(PendingInput::CardSelection(message));
            }
            CoreMessage::SelectYesNo(message) => {
                let label = get_card_text(message.card_code).unwrap();
                *pending_input = Some(PendingInput::Confirmation(
                    label
                        .optional_strings
                        .get(&message.string_index)
                        .unwrap_or(&String::from("undefined label"))
                        .to_owned(),
                ));
            }
            CoreMessage::SelectPosition(message) => {
                *pending_input = Some(PendingInput::BattlePositionSelection(message.positions));
            }
            CoreMessage::SelectUnselectCard(message) => {
                *pending_input = Some(PendingInput::CardToggleSelection(message));
            }
            CoreMessage::SelectTribute(message) => {
                *pending_input = Some(PendingInput::TributeSelection {
                    message,
                    selected: Vec::new(),
                });
            }
            CoreMessage::SelectOption(message) => {
                *pending_input = Some(PendingInput::OptionSelection(message));
            }
            CoreMessage::AnnounceNumber(message) => {
                *pending_input = Some(PendingInput::NumberSelection(message));
            }
            CoreMessage::SortCard(message) => {
                *pending_input = Some(PendingInput::CardOrdering(message));
            }
            _ => {}
        }
        false
    }

    pub fn refresh_queried_card_data(&self, cards: &mut CardTracker) -> anyhow::Result<()> {
        refresh_queried_card_data(cards, &self.duel)
    }
}

pub(crate) fn process_until_blocked(
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

pub(crate) async fn cache_duel_scripts() -> anyhow::Result<Scripts> {
    let deck_codes = MAIN_DECK_CODES
        .into_iter()
        .chain(EXTRA_DECK_CODES)
        .collect::<Vec<_>>();
    cache_scripts(&deck_codes).await
}

fn move_card_codes_to_front(main_deck: &mut [u32; 40], target_codes: &[u32]) {
    let mut front_index = 0;
    for &target_code in target_codes {
        if let Some(pos) = main_deck[front_index..]
            .iter()
            .position(|item| *item == target_code)
        {
            let actual_idx = front_index + pos;
            main_deck[front_index..=actual_idx].rotate_right(1);
            front_index += 1;
        }
    }
}

pub(crate) async fn load_core(scripts: Scripts) -> anyhow::Result<OCGCore> {
    OCGCore::load(
        read_card_definition,
        Box::new(move |name: &str| scripts.get(name).cloned()),
        log_core_message,
    )
    .await
}

pub(crate) async fn load_duel(
    core: OCGCore,
    custom_hand: Option<String>,
) -> anyhow::Result<DuelSetup> {
    let deck_codes = MAIN_DECK_CODES
        .into_iter()
        .chain(EXTRA_DECK_CODES)
        .collect::<Vec<_>>();
    let archetype_script = core
        .script(ARCHETYPE_SCRIPT)
        .ok_or_else(|| anyhow::anyhow!("{ARCHETYPE_SCRIPT} is not loaded"))?;
    store_card_definitions(fetch_card_definitions(deck_codes, &archetype_script).await?);

    let mut main_deck = MAIN_DECK_CODES;
    main_deck.shuffle(&mut rand::rng());
    let mut starting_draw_count = 5u32;
    if let Some(card_list) = custom_hand
        && !card_list.is_empty()
    {
        let parsed_custom_hand: CustomHand = CustomHand::try_from(card_list)?;
        starting_draw_count = parsed_custom_hand.0.len() as u32;
        move_card_codes_to_front(&mut main_deck, parsed_custom_hand.0.as_slice());
        main_deck.reverse();
    }
    let cards = CardTracker::from_decks(&main_deck, &EXTRA_DECK_CODES)?;
    let duel = core.create_duel(starting_draw_count)?;
    for name in ["constant.lua", "utility.lua"] {
        let script = core
            .script(name)
            .ok_or_else(|| anyhow::anyhow!("{name} is not loaded"))?;
        core.load_script(&duel, &script, name)?;
    }
    for card_code in main_deck {
        duel.add_card(
            Player::Local,
            card_code,
            Player::Local,
            CardLocation::Deck,
            0,
            0,
        );
    }
    for card_code in EXTRA_DECK_CODES {
        duel.add_card(
            Player::Local,
            card_code,
            Player::Local,
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

fn refresh_queried_card_data(tracker: &mut CardTracker, duel: &Duel) -> anyhow::Result<()> {
    let player = Player::Local;
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
        let addresses: Vec<_> = tracker
            .cards()
            .iter()
            .filter_map(|card| match card.placement {
                CardPlacement::Zone(address)
                    if address.controller == player && address.location == location =>
                {
                    Some(address)
                }
                _ => None,
            })
            .collect();
        for address in addresses {
            let data = core_cards
                .get(address.sequence as usize)
                .and_then(|card| *card)
                .with_context(|| format!("No queried card at {address:?}"))?;
            tracker.update_queried_data(address, data)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/duel/controller.rs"]
mod tests;
