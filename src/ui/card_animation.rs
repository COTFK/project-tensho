use crate::ocgcore::constants::{CardLocation, Player};
use crate::ui::card_layer::{CardId, CardLayer, CardPlacement};
use dioxus::prelude::*;

pub(crate) const ANIMATION_MILLIS: u32 = 300;
// Allow the CSS transition to finish before unblocking input.
const COMPLETION_MARGIN_MILLIS: u32 = 50;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardAnimationState {
    pub active: bool,
    // Invalidates timers from cancelled or superseded animations.
    pub generation: u64,
    pub moving: Vec<CardId>,
    // Previous state for preserving pile images during movement.
    pub source: Option<CardLayer>,
}

impl CardAnimationState {
    pub fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active = false;
        self.moving.clear();
        self.source = None;
    }

    pub fn begin(&mut self, moving: Vec<CardId>, source: CardLayer) {
        self.generation = self.generation.wrapping_add(1);
        self.moving = moving;
        self.source = Some(source);
        self.active = true;
    }

    pub fn finish(&mut self, generation: u64) {
        if self.active && self.generation == generation {
            self.active = false;
            self.moving.clear();
            self.source = None;
        }
    }
}

fn visible(layer: &CardLayer, placement: CardPlacement) -> bool {
    let address = match placement {
        CardPlacement::Zone(address) => address,
        CardPlacement::Overlay { host, .. } => {
            let Some(host) = layer.card_by_id(host) else {
                return false;
            };
            let CardPlacement::Zone(address) = host.placement else {
                return false;
            };
            address
        }
    };
    address.controller == Player::Player
        && matches!(
            address.location,
            CardLocation::Deck
                | CardLocation::ExtraDeck
                | CardLocation::Graveyard
                | CardLocation::Banishment
                | CardLocation::Hand
                | CardLocation::MonsterZone
                | CardLocation::SpellTrapZone
        )
}

pub(crate) fn movements(before: &CardLayer, after: &CardLayer) -> Vec<CardId> {
    // Animate only cards visible at both ends of a transition.
    before
        .cards()
        .iter()
        .filter_map(|old| {
            let new = after.card_by_id(old.id)?;
            ((old.placement != new.placement || old.position != new.position)
                && visible(before, old.placement)
                && visible(after, new.placement))
            .then_some(old.id)
        })
        .collect()
}

pub(crate) fn use_animation_completion(mut animation: Signal<CardAnimationState>) {
    use_effect(move || {
        let current = animation();
        if current.active {
            spawn(async move {
                wait_for_animation().await;
                animation.write().finish(current.generation);
            });
        }
    });
}

async fn wait_for_animation() {
    wait_millis(ANIMATION_MILLIS + COMPLETION_MARGIN_MILLIS).await;
}

pub(crate) async fn wait_millis(millis: u32) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(millis).await;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (sender, receiver) = futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(u64::from(millis)));
            if sender.send(()).is_err() {
                tracing::debug!("Timer was cancelled");
            }
        });
        if let Err(error) = receiver.await {
            tracing::error!("Timer failed: {error}");
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/ui/card_animation.rs"]
mod tests;
