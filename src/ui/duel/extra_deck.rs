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
use crate::ui::components::svg::SummonIcon;

#[component]
pub fn ExtraDeckZone(has_cards: bool, highlighted: bool, onbrowse: EventHandler<()>) -> Element {
    rsx!(
        div {
            class: "relative bg-slate-50/2 {ZONE_SIZE} aspect-square flex items-center justify-center border-0.5",
            class: if has_cards {"hover:outline-4 hover:outline-yellow-300"},
            onclick: move |_| if has_cards {
                onbrowse.call(());
            },
            if has_cards {
                div {
                    class: "relative h-full aspect-[59/86]",
                    div {
                        class: "absolute inset-[1px] my-0.5 ml-0.5 mb-1 md:inset-[2px] md:my-1 md:mb-2 md:ml-1 rounded-[2px] blur-[1px] mix-blend-screen pointer-events-none",
                        class: if highlighted { "bg-yellow-400" },
                    }
                }
            }
        }
    )
}

#[component]
pub fn ExtraDeckModal(
    extra_deck: Vec<PileCardRowData>,
    available_special_summons: Vec<DuelCardData>,
    mut open_pile: Signal<Option<CardLocation>>,
    suppress_actions: bool,
    onaction: EventHandler<CardIntent>,
) -> Element {
    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Extra Deck",
            trigger: open_pile() == Some(CardLocation::ExtraDeck),
            retain_on_close: true,
            div {
                class: PILE_LIST_CLASS,
                for (index, card) in extra_deck.iter().enumerate() {
                    {
                        let data = card.data;
                        let is_special_summonable = available_special_summons
                            .iter()
                            .any(|summon| {
                                summon.location == CardLocation::ExtraDeck
                                    && summon.sequence == data.sequence
                            });

                        rsx!(
                            PileCardRow {
                                card: card.clone(),
                                is_selected: selected_card() == Some(index),
                                highlighted: is_special_summonable && !suppress_actions,
                                show_actions: selected_card() == Some(index) && is_special_summonable && !suppress_actions,
                                onselect: move |_| selected_card.set(Some(index)),
                                LabeledIconButton {
                                    label: "Summon",
                                    class: "border-yellow-500 text-yellow-300",
                                    onclick: move |_| {
                                        if is_special_summonable {
                                            onaction.call(CardIntent::SpecialSummon(CardAddress {
                                                controller: data.controller,
                                                location: CardLocation::ExtraDeck,
                                                sequence: u32::from(data.sequence),
                                            }));
                                        }
                                        selected_card.set(None);
                                        open_pile.set(None);
                                    },
                                    SummonIcon {  }
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
