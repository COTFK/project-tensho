use dioxus::prelude::*;

use super::card_animation::CardAnimationState;
use super::chain::ChainPlayback;

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Playback {
    pub animation: Signal<CardAnimationState>,
    pub chain: Signal<ChainPlayback>,
}

impl Playback {
    pub fn is_active(&self) -> bool {
        self.animation.read().is_active() || self.chain.read().active
    }

    pub fn cancel(&mut self) {
        self.animation.write().cancel();
        self.chain.write().cancel();
    }
}

#[cfg(test)]
pub(crate) fn use_playback() -> Playback {
    use_context_provider(Playback::default)
}
