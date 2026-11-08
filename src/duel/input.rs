use crate::duel::cards::CardAddress;
use crate::ocgcore::constants::{BattlePosition, CardLocation};
use crate::ocgcore::messages::{
    AnnounceNumberMessageData, IdleMessageData, SelectCardMessageData, SelectChainMessageData,
    SelectOptionMessageData, SelectTributeMessageData, SelectUnselectMessageData,
    SortCardMessageData,
};
use crate::ocgcore::{CardData, Response, Zone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingInput {
    Idle(IdleMessageData),
    Place(Vec<Zone>),
    Chain(SelectChainMessageData),
    EffectYesNo(CardData),
    Cards(SelectCardMessageData),
    YesNo(String),
    Position(Vec<BattlePosition>),
    Unselect(SelectUnselectMessageData),
    Tribute {
        message: SelectTributeMessageData,
        selected: Vec<u8>,
    },
    Options(SelectOptionMessageData),
    Numbers(AnnounceNumberMessageData),
    Sort(SortCardMessageData),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CardAction {
    Select(CardAddress),
    Summon(CardAddress),
    Activate(CardAddress),
    Set(CardAddress),
    SpecialSummon(CardAddress),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardActions {
    pub summon: Option<u8>,
    pub special_summon: Option<u8>,
    pub monster_set: Option<u8>,
    pub spell_trap_set: Option<u8>,
    pub effects: Vec<CardData>,
    pub prompted: Option<CardData>,
    pub selection: Option<u32>,
    pub selectable: bool,
    pub selected: bool,
    pub tribute: Option<u8>,
    pub tribute_selectable: bool,
    pub tribute_selected: bool,
    pub tribute_max: usize,
}

impl PendingInput {
    pub fn prompted_effects(&self) -> &[CardData] {
        match self {
            Self::Chain(message) => &message.effects,
            Self::EffectYesNo(card) => std::slice::from_ref(card),
            _ => &[],
        }
    }

    pub fn decline_response(&self) -> Option<Response> {
        match self {
            Self::Chain(message) if !message.effects.is_empty() => Some(
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
            Self::EffectYesNo(_) | Self::YesNo(_) => Some(Response::No),
            Self::Tribute { message, .. } if message.is_cancelable => Some(Response::PassPriority),
            _ => None,
        }
    }

    pub fn card_actions(&self, address: CardAddress) -> CardActions {
        let matches = |card: &&CardData| {
            card.controller == address.controller
                && card.location == address.location
                && u32::from(card.sequence) == address.sequence
        };
        let mut actions = CardActions::default();
        match self {
            Self::Idle(message) => {
                let last_index = |cards: &[CardData]| {
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
            Self::Chain(_) | Self::EffectYesNo(_) => {
                let mut effects = self.prompted_effects().iter().filter(matches);
                actions.prompted = if address.location == CardLocation::Hand {
                    effects.last().copied()
                } else {
                    effects.next().copied()
                };
            }
            Self::Unselect(message) => {
                let card =
                    message.select_card_for(address.controller, address.location, address.sequence);
                actions.selection = card.and_then(|card| message.response_index_for(&card));
                actions.selectable = card.is_some();
                actions.selected = card.is_some_and(|card| card.is_selected);
            }
            Self::Tribute { message, selected } => {
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
