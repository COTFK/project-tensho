use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{Context, ensure};
use dioxus::html::geometry::PixelsRect;
use dioxus::prelude::*;

use crate::duel::cards::CardAddress;
use crate::ocgcore::constants::{CardLocation, Player};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) enum LayoutAnchorId {
    Zone(CardLocation, u32),
    HandBase,
    Pile(CardLocation),
}

impl TryFrom<CardAddress> for LayoutAnchorId {
    type Error = anyhow::Error;

    fn try_from(address: CardAddress) -> anyhow::Result<Self> {
        ensure!(
            address.controller == Player::Local,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AnchorGeometry {
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

impl AnchorGeometry {
    fn from_rect(rect: PixelsRect) -> anyhow::Result<Self> {
        if rect.size.width == 0.0 || rect.size.height == 0.0 {
            return Err(AnchorNotReady("Card anchor has no size yet").into());
        }
        ensure!(
            rect.size.width.is_finite()
                && rect.size.height.is_finite()
                && rect.origin.x.is_finite()
                && rect.origin.y.is_finite()
                && rect.size.width > 0.0
                && rect.size.height > 0.0,
            "Invalid or hidden card anchor"
        );
        Ok(Self {
            center_x: rect.origin.x + rect.size.width / 2.0,
            center_y: rect.origin.y + rect.size.height / 2.0,
            width: rect.size.width,
            height: rect.size.height,
        })
    }
}

type AnchorEntry = (Rc<()>, Rc<MountedData>);

#[derive(Clone, Default)]
pub(super) struct LayoutAnchorRegistry(Rc<RefCell<HashMap<LayoutAnchorId, AnchorEntry>>>);

impl LayoutAnchorRegistry {
    pub(super) fn insert(&self, anchor: LayoutAnchorId, owner: Rc<()>, element: Rc<MountedData>) {
        self.0.borrow_mut().insert(anchor, (owner, element));
    }

    fn remove(&self, anchor: LayoutAnchorId, owner: &Rc<()>) {
        let mut anchors = self.0.borrow_mut();
        if anchors
            .get(&anchor)
            .is_some_and(|entry| Rc::ptr_eq(&entry.0, owner))
        {
            anchors.remove(&anchor);
        }
    }

    pub async fn measure(&self, address: CardAddress) -> anyhow::Result<AnchorGeometry> {
        self.measure_anchor(LayoutAnchorId::try_from(address)?)
            .await
    }

    pub async fn measure_hand_base(&self) -> anyhow::Result<AnchorGeometry> {
        self.measure_anchor(LayoutAnchorId::HandBase).await
    }

    async fn measure_anchor(&self, anchor: LayoutAnchorId) -> anyhow::Result<AnchorGeometry> {
        let (missing, measuring, changed) = match anchor {
            LayoutAnchorId::HandBase => (
                "Hand anchor not mounted",
                "Measuring hand anchor",
                "Hand anchor changed while measuring",
            ),
            _ => (
                "Card anchor not mounted",
                "Measuring card anchor",
                "Card anchor changed while measuring",
            ),
        };
        let element = {
            let anchors = self.0.borrow();
            anchors
                .get(&anchor)
                .context(AnchorNotReady(missing))?
                .1
                .clone()
        };
        let rect = element
            .get_client_rect()
            .await
            .map_err(|error| anyhow::anyhow!("{measuring}: {error}"))?;
        let anchors = self.0.borrow();
        ensure!(
            anchors
                .get(&anchor)
                .is_some_and(|entry| Rc::ptr_eq(&entry.1, &element)),
            changed
        );
        AnchorGeometry::from_rect(rect)
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
pub(super) fn LayoutAnchor(anchor: LayoutAnchorId, class: &'static str) -> Element {
    let anchors = use_context::<LayoutAnchorRegistry>();
    let revision = use_context::<LayoutRevision>();
    let owner = use_hook(|| Rc::new(()));
    use_drop({
        let anchors = anchors.clone();
        let owner = owner.clone();
        move || anchors.remove(anchor, &owner)
    });
    rsx! {
        div {
            class,
            "aria-hidden": "true",
            div {
                class: "w-full h-full",
                onmounted: move |event| {
                    anchors.insert(anchor, owner.clone(), event.data());
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
