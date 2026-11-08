use crate::duel::cards::{CardId, CardPlacement, CardTracker};
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::messages::CoreMessage;
use dioxus::prelude::*;

pub(crate) const ANIMATION_MILLIS: u32 = 300;
// Allow the CSS transition to finish before unblocking input.
const COMPLETION_MARGIN_MILLIS: u32 = 50;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AnimationPhase {
    #[default]
    Idle,
    Moving,
    HandShuffleGathering,
    HandShuffleStacking,
    HandShuffleSpreading,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardAnimationState {
    pub phase: AnimationPhase,
    // Invalidates timers from cancelled or superseded animations.
    pub generation: u64,
    pub moving: Vec<CardId>,
    // Previous state for preserving pile images during movement.
    pub source: Option<CardTracker>,
}

impl CardAnimationState {
    pub fn is_active(&self) -> bool {
        self.phase != AnimationPhase::Idle
    }

    pub fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.phase = AnimationPhase::Idle;
        self.moving.clear();
        self.source = None;
    }

    pub fn begin(&mut self, moving: Vec<CardId>, source: CardTracker) {
        self.generation = self.generation.wrapping_add(1);
        self.moving = moving;
        self.source = Some(source);
        self.phase = AnimationPhase::Moving;
    }

    pub fn begin_gather(&mut self, mut moving: Vec<CardId>, source: CardTracker) {
        moving.extend(source.cards().iter().filter_map(|card| {
            matches!(card.placement, CardPlacement::Zone(address)
                if address.controller == Player::Player && address.location == CardLocation::Hand)
            .then_some(card.id)
        }));
        self.begin(moving, source);
        self.phase = AnimationPhase::HandShuffleGathering;
    }

    pub(crate) fn advance(&mut self, generation: u64, phase: AnimationPhase) {
        if self.generation != generation || self.phase != phase {
            return;
        }
        match phase {
            AnimationPhase::HandShuffleGathering => {
                self.phase = AnimationPhase::HandShuffleStacking
            }
            AnimationPhase::HandShuffleStacking => {
                self.phase = AnimationPhase::HandShuffleSpreading
            }
            AnimationPhase::Moving | AnimationPhase::HandShuffleSpreading => {
                self.finish(generation)
            }
            AnimationPhase::Idle => {}
        }
    }

    pub fn finish(&mut self, generation: u64) {
        if self.is_active() && self.generation == generation {
            self.phase = AnimationPhase::Idle;
            self.moving.clear();
            self.source = None;
        }
    }
}

fn visible(layer: &CardTracker, placement: CardPlacement) -> bool {
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CardAnimation {
    Movement(Vec<CardId>),
    HandShuffle(Vec<CardId>),
}

impl CardAnimation {
    pub fn begin(self, state: &mut CardAnimationState, source: CardTracker) {
        match self {
            Self::Movement(moving) => state.begin(moving, source),
            Self::HandShuffle(moving) => state.begin_gather(moving, source),
        }
    }
}

pub(crate) fn plan_animation(
    messages: &[CoreMessage],
    before: &CardTracker,
    after: &CardTracker,
) -> Option<CardAnimation> {
    let mut moving = movements(before, after);
    if messages.iter().any(|message| {
        matches!(message, CoreMessage::ShuffleHand(data)
            if data.player == Player::Player && data.card_codes.len() > 1)
    }) {
        if let Some(shuffled) = reshuffled_hand(before, after) {
            moving.extend(shuffled);
            return Some(CardAnimation::HandShuffle(moving));
        }
    }
    (!moving.is_empty()).then_some(CardAnimation::Movement(moving))
}

fn movements(before: &CardTracker, after: &CardTracker) -> Vec<CardId> {
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

fn reshuffled_hand(before: &CardTracker, after: &CardTracker) -> Option<Vec<CardId>> {
    let hand = |layer: &CardTracker| {
        layer
            .cards()
            .iter()
            .filter(|card| {
                matches!(card.placement, CardPlacement::Zone(address)
                if address.controller == Player::Player && address.location == CardLocation::Hand)
            })
            .map(|card| card.id)
            .collect::<Vec<_>>()
    };
    let old = hand(before);
    let new = hand(after);
    (old.len() > 1 && !new.is_empty() && old.iter().all(|id| after.card_by_id(*id).is_none()))
        .then_some(new)
}

pub(crate) fn use_animation_completion(mut animation: Signal<CardAnimationState>) {
    use_effect(move || {
        let current = animation();
        if current.is_active() {
            spawn(async move {
                if current.phase == AnimationPhase::HandShuffleStacking {
                    // Give newly keyed cards a frame at the stack before spreading.
                    wait_millis(COMPLETION_MARGIN_MILLIS).await;
                } else {
                    wait_for_animation().await;
                }
                let state = animation.peek();
                let ready = state.generation == current.generation && state.phase == current.phase;
                drop(state);
                if ready {
                    animation.write().advance(current.generation, current.phase);
                }
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
