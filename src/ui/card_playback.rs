use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ui::card_layer::{CardId, CardLayer, CardPlacement};

pub(crate) const ANIMATION_MILLIS: u32 = 300;
const COMPLETION_MARGIN_MILLIS: u32 = 50;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardPlayback {
    pub enabled: bool,
    pub active: bool,
    pub finished: bool,
    pub generation: u64,
    pub moving: Vec<CardId>,
    pub source: Option<CardLayer>,
    pub piles: [Vec<Option<CardData>>; 3],
    pub reduced_motion: bool,
    pub layout_pending: bool,
    pub restarting: bool,
    pub failure: Option<String>,
}

impl Default for CardPlayback {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            finished: false,
            generation: 0,
            moving: Vec::new(),
            source: None,
            piles: Default::default(),
            reduced_motion: true,
            layout_pending: false,
            restarting: false,
            failure: None,
        }
    }
}

impl CardPlayback {
    pub fn blocked(&self) -> bool {
        self.active || self.layout_pending || self.restarting || self.failure.is_some()
    }

    fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active = false;
        self.moving.clear();
        self.source = None;
        self.piles = Default::default();
    }

    pub fn begin(
        &mut self,
        moving: Vec<CardId>,
        source: CardLayer,
        piles: [Vec<Option<CardData>>; 3],
    ) {
        self.generation = self.generation.wrapping_add(1);
        self.moving = moving;
        self.source = Some(source);
        self.piles = piles;
        self.active = true;
    }

    pub fn finish(&mut self, generation: u64) {
        if self.active && self.generation == generation {
            self.active = false;
            self.moving.clear();
            self.source = None;
            self.piles = Default::default();
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.cancel();
            self.layout_pending = false;
        }
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        if reduced && self.active {
            self.cancel();
        }
    }

    pub fn resize(&mut self) {
        if self.enabled && self.active {
            self.cancel();
            self.layout_pending = true;
        }
    }

    pub fn restart(&mut self) {
        self.cancel();
        self.layout_pending = false;
        self.restarting = true;
    }

    pub fn fail(&mut self, error: String) {
        self.cancel();
        self.layout_pending = false;
        self.failure = Some(error);
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

pub(crate) async fn wait_for_animation() {
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
#[path = "../../tests/unit/ui/card_playback.rs"]
mod tests;
