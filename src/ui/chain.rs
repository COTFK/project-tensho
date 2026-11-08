use std::collections::VecDeque;

use dioxus::prelude::*;

use crate::duel::cards::CardId;
use crate::duel::controller::{ChainEvent, ChainLink};
use crate::ui::card_animation::wait_millis;

const CHAIN_FRAME_MILLIS: u32 = 600;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChainPlayback {
    links: Vec<ChainLink>,
    pub displayed: Vec<ChainLink>,
    frames: VecDeque<Vec<ChainLink>>,
    pub active: bool,
    generation: u64,
}

impl ChainPlayback {
    pub fn links_for_card(&self, card: CardId) -> Vec<u32> {
        self.displayed
            .iter()
            .filter_map(|link| (link.card == Some(card)).then_some(link.number))
            .collect()
    }

    pub fn has_pending_frames(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn record(&mut self, event: ChainEvent) {
        match event {
            ChainEvent::Added(link) => {
                if self
                    .links
                    .iter()
                    .any(|existing| existing.number == link.number)
                {
                    tracing::warn!("Replacing duplicate chain link {}", link.number);
                    self.links.retain(|existing| existing.number != link.number);
                }
                self.links.push(link);
            }
            ChainEvent::Solved(number) => {
                let old_len = self.links.len();
                self.links.retain(|link| link.number != number);
                if self.links.len() == old_len {
                    tracing::warn!("Solved unknown chain link {}", number);
                    return;
                }
            }
            ChainEvent::End => {
                if self.links.is_empty() {
                    return;
                }
                self.links.clear();
            }
        }
        self.frames.push_back(self.links.clone());
    }

    pub fn start(&mut self) {
        if self.active {
            return;
        }
        if let Some(frame) = self.frames.pop_front() {
            self.displayed = frame;
            if self.displayed.is_empty() && self.frames.is_empty() {
                return;
            }
            self.active = true;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub(crate) fn advance(&mut self, generation: u64) {
        if !self.active || self.generation != generation {
            return;
        }
        if let Some(frame) = self.frames.pop_front() {
            self.displayed = frame;
            if !self.displayed.is_empty() || !self.frames.is_empty() {
                self.generation = self.generation.wrapping_add(1);
                return;
            }
        }
        self.active = false;
    }

    pub fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active = false;
        self.links.clear();
        self.displayed.clear();
        self.frames.clear();
    }
}

pub(crate) fn use_chain_playback(mut playback: Signal<ChainPlayback>) {
    use_effect(move || {
        let state = playback();
        if state.active {
            spawn(async move {
                wait_millis(CHAIN_FRAME_MILLIS).await;
                let current = playback.peek();
                let ready = current.active && current.generation == state.generation;
                drop(current);
                if ready {
                    playback.write().advance(state.generation);
                }
            });
        }
    });
}

#[cfg(test)]
#[path = "../../tests/unit/ui/chain.rs"]
mod tests;
