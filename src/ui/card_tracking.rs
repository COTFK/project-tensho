use anyhow::{Context, ensure};
use ocgcore_ffi::constants::{MSG_RELOAD_FIELD, MSG_TAG_SWAP};

use crate::ocgcore::Duel;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::messages::CoreMessage;
use crate::ui::card_layer::{CardAddress, CardLayer, CardPlacement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardTracking {
    Active(CardLayer),
    Failed(String),
}

impl CardTracking {
    pub fn apply(&mut self, message: &CoreMessage) {
        let Self::Active(layer) = self else {
            return;
        };
        let result = match message {
            CoreMessage::Move(_)
            | CoreMessage::Swap(_)
            | CoreMessage::SwapGraveDeck(_)
            | CoreMessage::RemoveCards(_)
            | CoreMessage::Draw(_)
            | CoreMessage::ReverseDeck
            | CoreMessage::DeckTop(_)
            | CoreMessage::ConfirmDeckTop(_)
            | CoreMessage::ConfirmExtraTop(_)
            | CoreMessage::ShuffleDeck(_)
            | CoreMessage::ShuffleHand(_)
            | CoreMessage::ShuffleExtra(_)
            | CoreMessage::ShuffleSetCard(_)
            | CoreMessage::PositionChange(_)
            | CoreMessage::SetCard(_)
            | CoreMessage::Summoning(_)
            | CoreMessage::SpecialSummoning(_)
            | CoreMessage::FlipSummoning(_) => layer.apply(message),
            CoreMessage::Unsupported(kind)
                if matches!(u32::from(*kind), MSG_TAG_SWAP | MSG_RELOAD_FIELD) =>
            {
                Err(anyhow::anyhow!(
                    "Unsupported structural core message {kind}"
                ))
            }
            _ => return,
        };
        if let Err(error) = result.and_then(|()| layer.validate()) {
            self.fail(format!("Card tracking failed for {message:?}: {error:#}"));
        }
    }

    pub fn reconcile(&mut self, duel: &Duel) {
        let Self::Active(layer) = self else {
            return;
        };
        if let Err(error) = Self::reconcile_layer(layer, duel) {
            self.fail(format!("Card tracking diverged from the core: {error:#}"));
        }
    }

    fn reconcile_layer(layer: &mut CardLayer, duel: &Duel) -> anyhow::Result<()> {
        layer.validate()?;
        for player in [Player::Player, Player::Opponent] {
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
                let tracked_count = layer
                    .cards()
                    .iter()
                    .filter(|card| {
                        matches!(
                            card.placement,
                            CardPlacement::Zone(address)
                                if address.controller == player && address.location == location
                        )
                    })
                    .count();
                ensure!(
                    tracked_count == core_cards.iter().flatten().count(),
                    "{player:?} {location} card count differs"
                );
                for (sequence, core_card) in core_cards.iter().enumerate() {
                    let address = CardAddress {
                        controller: player,
                        location,
                        sequence: u32::try_from(sequence)?,
                    };
                    match (layer.card_at(address), core_card) {
                        (None, None) => {}
                        (Some(card), Some(core_card)) => {
                            ensure!(
                                card.code == 0
                                    || core_card.card_code == 0
                                    || card.code == core_card.card_code,
                                "{player:?} {location} sequence {sequence} has a different card code"
                            );
                            if location == CardLocation::Graveyard {
                                layer.reconcile_graveyard_code(address, core_card.card_code)?;
                            }
                        }
                        _ => anyhow::bail!(
                            "{player:?} {location} sequence {sequence} occupancy differs"
                        ),
                    }
                }
            }
        }
        Ok(())
    }

    fn fail(&mut self, error: String) {
        tracing::error!("{error}");
        *self = Self::Failed(error);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/ui/card_tracking.rs"]
mod tests;
