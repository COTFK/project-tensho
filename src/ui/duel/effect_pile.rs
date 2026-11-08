use dioxus::prelude::*;

use super::constants::ZONE_SIZE;
use super::pile_cards::{PILE_LIST_CLASS, PileCardRow, PileCardRowData};
use crate::duel::cards::CardAddress;
use crate::duel::input::CardIntent;
use crate::ocgcore::DuelCardData;
use crate::ocgcore::constants::CardLocation;
use crate::ui::components::LabeledIconButton;
use crate::ui::components::PickerModal;
use crate::ui::components::TextButton;
use crate::ui::components::svg::LightningTrioIcon;

#[component]
pub fn EffectPileZone(
    location: CardLocation,
    has_cards: bool,
    highlighted: bool,
    onbrowse: EventHandler<CardLocation>,
) -> Element {
    rsx!(
        div {
            class: if location == CardLocation::Banishment {
                "relative w-full h-full flex items-center justify-center border-0.5"
            } else {
                "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5"
            },
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards {
                onbrowse.call(location);
            },
            if has_cards {
                div {
                    class: if location == CardLocation::Banishment {
                        "relative h-full aspect-[59/86] -rotate-90"
                    } else {
                        "relative h-full aspect-[59/86]"
                    },
                    div {
                        class: "absolute inset-[2px] md:inset-[4px] my-0.5 md:my-1 rounded-[2px] blur-[1px] mix-blend-screen pointer-events-none",
                        class: if highlighted { "bg-yellow-400" },
                    }
                }
            }
        }
    )
}

#[component]
pub fn EffectPileModal(
    location: CardLocation,
    cards: Vec<PileCardRowData>,
    cards_prompting_to_activate: Vec<DuelCardData>,
    mut open_pile: Signal<Option<CardLocation>>,
    suppress_actions: bool,
    onaction: EventHandler<CardIntent>,
) -> Element {
    let title = if location == CardLocation::Banishment {
        "Banishment"
    } else {
        "Graveyard"
    };

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title,
            trigger: open_pile() == Some(location),
            retain_on_close: true,
            div {
                class: PILE_LIST_CLASS,
                for (index, card) in cards.iter().enumerate() {
                    {
                        let prompted_card = cards_prompting_to_activate
                            .iter()
                            .find(|card| card.location == location && card.sequence == index as u8)
                            .copied();
                        let address = CardAddress {
                            controller: card.data.controller,
                            location,
                            sequence: u32::from(card.data.sequence),
                        };

                        rsx!(
                            PileCardRow {
                                card: card.clone(),
                                is_selected: selected_card() == Some(index),
                                highlighted: prompted_card.is_some() && !suppress_actions,
                                show_actions: selected_card() == Some(index) && prompted_card.is_some() && !suppress_actions,
                                onselect: move |_| selected_card.set(Some(index)),
                                LabeledIconButton {
                                    label: "Activate",
                                    class: "border-yellow-500 text-yellow-300",
                                    onclick: move |_| {
                                        if prompted_card.is_some() && !suppress_actions {
                                            onaction.call(CardIntent::Activate(address));
                                            selected_card.set(None);
                                        }
                                    },
                                    LightningTrioIcon {  }
                                }
                            }
                        )
                    }

                }
            }
            TextButton {
                label: "Close",
                onclick: move |_| {
                    open_pile.set(None);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}
