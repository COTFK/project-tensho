use dioxus::prelude::*;

use super::chain_badges::ChainBadges;
use crate::ocgcore::CardData;
use crate::ui::components::{Card, CardActionMenu, CardImage};

pub(super) const PILE_LIST_CLASS: &str = "flex flex-row min-w-[40vw] w-full max-w-[77vw] h-max gap-0.5 px-2 overflow-x-auto scroll-smooth scrollbar-thin";

#[derive(Clone, PartialEq)]
pub(super) struct PileCard {
    pub data: CardData,
    pub links: Vec<u32>,
}

#[component]
pub(super) fn PileCardRow(
    card: PileCard,
    is_selected: bool,
    highlighted: bool,
    show_actions: bool,
    onselect: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "relative py-2",
            Card {
                image: CardImage::Face(card.data),
                class: "w-[12vw]",
                is_selected,
                show_highlight_on_select: true,
                show_dotted_highlight: false,
                show_blue_aura: false,
                show_orange_aura: highlighted,
                onclick: move |_| onselect.call(()),
            }
            ChainBadges { links: card.links }
            CardActionMenu {
                class: "absolute left-1/2 bottom-1/2 -translate-x-[50%] translate-y-[50%] px-3 py-2 md:px-6 z-20",
                trigger: show_actions,
                {children}
            }
        }
    }
}
