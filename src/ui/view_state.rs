use dioxus::prelude::*;

use crate::define_ui_state;
use crate::duel::cards::{CardAddress, CardInstanceId, CardPlacement, CardTracker};
use crate::ocgcore::DuelCardData;
use crate::ocgcore::constants::{CardLocation, Player};

define_ui_state!(DuelViewState {
    selected_card: Option<CardInstanceId>,
    open_pile: Option<CardLocation>,
    effects_to_select_from: Vec<DuelCardData>,
});

impl DuelViewState {
    pub(crate) fn select_card(&mut self, address: CardAddress, cards: Option<&CardTracker>) {
        let id = match cards {
            Some(cards) => match cards.card_at(address) {
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
        self.selected_card.set(Some(id));
    }

    pub(crate) fn is_selected(&self, address: CardAddress, cards: Option<&CardTracker>) -> bool {
        let selected = (self.selected_card)();
        cards
            .and_then(|cards| cards.card_at(address))
            .is_some_and(|card| Some(card.id) == selected)
    }

    pub(crate) fn reconcile_selection(&mut self, cards: &CardTracker) {
        let Some(id) = *self.selected_card.peek() else {
            return;
        };
        if !cards.card_by_id(id).is_some_and(|card| {
            matches!(card.placement, CardPlacement::Zone(address) if address.controller == Player::Local)
        }) {
            self.selected_card.set(None);
        }
    }
}
