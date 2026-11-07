use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{Context, ensure};
use dioxus::html::geometry::PixelsRect;
use dioxus::prelude::*;

use crate::ocgcore::constants::{CardLocation, Player};
use crate::ui::card_layer::CardAddress;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) enum Anchor {
    Zone(CardLocation, u32),
    HandBase,
    Pile(CardLocation),
}

impl TryFrom<CardAddress> for Anchor {
    type Error = anyhow::Error;

    fn try_from(address: CardAddress) -> anyhow::Result<Self> {
        ensure!(
            address.controller == Player::Player,
            "No opponent layout anchors"
        );
        match address.location {
            CardLocation::MonsterZone if address.sequence < 7 => {
                Ok(Self::Zone(address.location, address.sequence))
            }
            CardLocation::SpellTrapZone if address.sequence < 6 => {
                Ok(Self::Zone(address.location, address.sequence))
            }
            CardLocation::Deck
            | CardLocation::ExtraDeck
            | CardLocation::Graveyard
            | CardLocation::Banishment => Ok(Self::Pile(address.location)),
            _ => anyhow::bail!("No layout anchor for {address:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) enum AnchorPart {
    Card,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CardAnchor {
    pub center_x: f64,
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub(super) struct AnchorNotReady(&'static str);

impl std::fmt::Display for AnchorNotReady {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for AnchorNotReady {}

impl CardAnchor {
    fn from_rects(center: PixelsRect, size: PixelsRect) -> anyhow::Result<Self> {
        if center.size.width == 0.0
            || center.size.height == 0.0
            || size.size.width == 0.0
            || size.size.height == 0.0
        {
            return Err(AnchorNotReady("Card anchor has no size yet").into());
        }
        ensure!(
            center.size.width.is_finite()
                && center.size.height.is_finite()
                && center.origin.x.is_finite()
                && center.origin.y.is_finite()
                && center.size.width > 0.0
                && center.size.height > 0.0
                && size.size.width.is_finite()
                && size.size.height.is_finite()
                && size.size.width > 0.0
                && size.size.height > 0.0,
            "Invalid or hidden card anchor"
        );
        Ok(Self {
            center_x: center.origin.x + center.size.width / 2.0,
            center_y: center.origin.y + center.size.height / 2.0,
            width: size.size.width,
            height: size.size.height,
        })
    }
}

type AnchorKey = (Anchor, AnchorPart);
type AnchorEntry = (Rc<()>, Rc<MountedData>);

#[derive(Clone, Default)]
pub(super) struct LayoutAnchors(Rc<RefCell<HashMap<AnchorKey, AnchorEntry>>>);

impl LayoutAnchors {
    pub(super) fn insert(&self, key: AnchorKey, owner: Rc<()>, element: Rc<MountedData>) {
        self.0.borrow_mut().insert(key, (owner, element));
    }

    fn remove(&self, key: AnchorKey, owner: &Rc<()>) {
        let mut anchors = self.0.borrow_mut();
        if anchors
            .get(&key)
            .is_some_and(|entry| Rc::ptr_eq(&entry.0, owner))
        {
            anchors.remove(&key);
        }
    }

    pub async fn measure(&self, address: CardAddress) -> anyhow::Result<CardAnchor> {
        let anchor = Anchor::try_from(address)?;
        let key = (anchor, AnchorPart::Card);
        let element = {
            let anchors = self.0.borrow();
            anchors
                .get(&key)
                .context(AnchorNotReady("Card anchor not mounted"))?
                .1
                .clone()
        };
        let rect = element
            .get_client_rect()
            .await
            .map_err(|error| anyhow::anyhow!("Measuring card anchor: {error}"))?;
        let anchors = self.0.borrow();
        ensure!(
            anchors
                .get(&key)
                .is_some_and(|entry| Rc::ptr_eq(&entry.1, &element)),
            "Card anchor changed while measuring"
        );
        CardAnchor::from_rects(rect, rect)
    }

    pub async fn measure_hand_base(&self) -> anyhow::Result<CardAnchor> {
        let key = (Anchor::HandBase, AnchorPart::Card);
        let element = self
            .0
            .borrow()
            .get(&key)
            .context(AnchorNotReady("Hand anchor not mounted"))?
            .1
            .clone();
        let rect = element
            .get_client_rect()
            .await
            .map_err(|error| anyhow::anyhow!("Measuring hand anchor: {error}"))?;
        ensure!(
            self.0
                .borrow()
                .get(&key)
                .is_some_and(|entry| Rc::ptr_eq(&entry.1, &element)),
            "Hand anchor changed while measuring"
        );
        CardAnchor::from_rects(rect, rect)
    }
}

#[derive(Clone, Copy)]
pub(super) struct LayoutRevision(pub Signal<u64>);

impl LayoutRevision {
    pub fn invalidate(self) {
        let mut revision = self.0;
        revision += 1;
    }
}

#[component]
pub(super) fn LayoutAnchor(anchor: Anchor, part: AnchorPart, class: &'static str) -> Element {
    let anchors = use_context::<LayoutAnchors>();
    let revision = use_context::<LayoutRevision>();
    let owner = use_hook(|| Rc::new(()));
    let key = (anchor, part);
    use_drop({
        let anchors = anchors.clone();
        let owner = owner.clone();
        move || anchors.remove(key, &owner)
    });
    rsx! {
        div {
            class,
            "aria-hidden": "true",
            div {
                class: "w-full h-full",
                onmounted: move |event| {
                    anchors.insert(key, owner.clone(), event.data());
                    revision.invalidate();
                },
                onresize: move |_| revision.invalidate(),
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/layout_anchors.rs"]
mod tests;
