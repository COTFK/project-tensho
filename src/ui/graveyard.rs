use dioxus::prelude::*;

use super::card::ActivatableCard;
use super::components::BlockButton;
use super::components::PickerModal;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;

#[component]
pub fn Graveyard() -> Element {
    let mut state = use_context::<DuelState>();

    let any_trigger_effects_in_gy = state
        .card_prompting_to_activate
        .iter()
        .any(|card| card.location == CardLocation::Graveyard);

    rsx!(
        div {
            class: "relative shadow-xl bg-slate-50/2 size-[9vw] aspect-square flex items-center justify-center border-0.5 hover:outline-4 hover:outline-yellow-300",
            class: if any_trigger_effects_in_gy {"outline-4 outline-yellow-300/50"},
            onclick: move |_| state.show_graveyard.set(true),
            for (index, card) in (state.graveyard)().iter().enumerate() {
                div {
                    class: "absolute inset-[clamp(2px,0.6vw,8px)]",
                    img {
                        class: "w-full h-full object-contain",
                        style: "transform: translate({index as f32 * 0.01}vw, -{index as f32 * 0.01}vh);",
                        image_rendering: "smooth",
                        aspect_ratio: "59/86",
                        src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.unwrap().card_code),
                    }
                }
            }
        }
    )
}

#[component]
pub fn GraveyardModal() -> Element {
    let state = use_context::<DuelState>();
    let graveyard = state.graveyard;
    let mut show_graveyard = state.show_graveyard;

    let mut selected_card = use_signal(|| None);

    rsx!(
        PickerModal {
            title: "Graveyard",
            trigger: show_graveyard(),
            div {
                class: "flex flex-row gap-2",
                class: "overflow-x-auto scroll-smooth scrollbar-thin",
                for (index, card) in graveyard().iter().enumerate() {
                    ActivatableCard {
                        index: index as u8,
                        card: *card,
                        select_signal: selected_card,
                    }
                }
            }
            BlockButton {
                label: "Close",
                onclick: move |_| {
                    show_graveyard.set(false);
                    selected_card.set(None);
                },
                additional_classes: "bg-green-600/70"
            }
        }
    )
}
