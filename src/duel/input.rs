use crate::duel::cards::CardAddress;
use crate::ocgcore::constants::{BattlePosition, CardLocation};
use crate::ocgcore::messages::{
    AnnounceNumberMessageData, IdleMessageData, SelectCardMessageData, SelectChainMessageData,
    SelectOptionMessageData, SelectTributeMessageData, SelectUnselectMessageData,
    SortCardMessageData,
};
use crate::ocgcore::{DuelCardData, Response, Zone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingInput {
    ActionSelection(IdleMessageData),
    FieldPlacement(Vec<Zone>),
    ChainResponse(SelectChainMessageData),
    EffectConfirmation(DuelCardData),
    CardSelection(SelectCardMessageData),
    Confirmation(String),
    BattlePositionSelection(Vec<BattlePosition>),
    CardToggleSelection(SelectUnselectMessageData),
    TributeSelection {
        message: SelectTributeMessageData,
        selected: Vec<u8>,
    },
    OptionSelection(SelectOptionMessageData),
    NumberSelection(AnnounceNumberMessageData),
    CardOrdering(SortCardMessageData),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CardIntent {
    Select(CardAddress),
    Summon(CardAddress),
    Activate(CardAddress),
    Set(CardAddress),
    SpecialSummon(CardAddress),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardActionOptions {
    pub summon: Option<u8>,
    pub special_summon: Option<u8>,
    pub monster_set: Option<u8>,
    pub spell_trap_set: Option<u8>,
    pub effects: Vec<DuelCardData>,
    pub prompted: Option<DuelCardData>,
    pub selection: Option<u32>,
    pub selectable: bool,
    pub selected: bool,
    pub tribute: Option<u8>,
    pub tribute_selectable: bool,
    pub tribute_selected: bool,
    pub tribute_max: usize,
}

impl PendingInput {
    pub fn prompted_effects(&self) -> &[DuelCardData] {
        match self {
            Self::ChainResponse(message) => &message.effects,
            Self::EffectConfirmation(card) => std::slice::from_ref(card),
            _ => &[],
        }
    }

    pub fn decline_response(&self) -> Option<Response> {
        match self {
            Self::ChainResponse(message) if !message.effects.is_empty() => Some(
                if message
                    .effects
                    .iter()
                    .any(|card| card.action_index.is_some())
                {
                    Response::PassPriority
                } else {
                    Response::No
                },
            ),
            Self::EffectConfirmation(_) | Self::Confirmation(_) => Some(Response::No),
            Self::TributeSelection { message, .. } if message.is_cancelable => {
                Some(Response::PassPriority)
            }
            _ => None,
        }
    }

    pub fn card_action_options(&self, address: CardAddress) -> CardActionOptions {
        let matches = |card: &&DuelCardData| {
            card.controller == address.controller
                && card.location == address.location
                && u32::from(card.sequence) == address.sequence
        };
        let mut actions = CardActionOptions::default();
        match self {
            Self::ActionSelection(message) => {
                let last_index = |cards: &[DuelCardData]| {
                    cards
                        .iter()
                        .filter(matches)
                        .last()
                        .and_then(|card| card.action_index)
                };
                actions.summon = last_index(&message.normal_summons);
                actions.special_summon = message
                    .special_summons
                    .iter()
                    .position(|card| matches(&card))
                    .map(|index| index as u8);
                actions.monster_set = last_index(&message.monster_sets);
                actions.spell_trap_set = last_index(&message.spell_trap_sets);
                actions.effects = message
                    .activatable_effects
                    .iter()
                    .filter(matches)
                    .copied()
                    .collect();
            }
            Self::ChainResponse(_) | Self::EffectConfirmation(_) => {
                let mut effects = self.prompted_effects().iter().filter(matches);
                actions.prompted = if address.location == CardLocation::Hand {
                    effects.last().copied()
                } else {
                    effects.next().copied()
                };
            }
            Self::CardToggleSelection(message) => {
                let card =
                    message.select_card_for(address.controller, address.location, address.sequence);
                actions.selection = card.and_then(|card| message.response_index_for(&card));
                actions.selectable = card.is_some();
                actions.selected = card.is_some_and(|card| card.is_selected);
            }
            Self::TributeSelection { message, selected } => {
                let card = message.candidates.iter().find(matches);
                actions.tribute = card.and_then(|card| card.action_index);
                actions.tribute_selectable = card.is_some();
                actions.tribute_selected = actions
                    .tribute
                    .is_some_and(|index| selected.contains(&index));
                actions.tribute_max = message.max_select as usize;
            }
            _ => {}
        }
        actions
    }
}

#[cfg(test)]
#[path = "../../tests/unit/duel/input.rs"]
mod tests;
