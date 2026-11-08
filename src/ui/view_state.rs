use dioxus::prelude::*;

use crate::define_ui_state;
use crate::duel::cards::{CardAddress, CardId, CardPlacement, CardTracker};
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};

define_ui_state!(ViewState {
    selected_card: Option<CardId>,
    open_pile: Option<CardLocation>,
    effects_to_select_from: Vec<CardData>,
});

impl ViewState {
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
            matches!(card.placement, CardPlacement::Zone(address) if address.controller == Player::Player)
        }) {
            self.selected_card.set(None);
        }
    }
}
