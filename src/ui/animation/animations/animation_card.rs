use dioxus::prelude::*;

use crate::ocgcore::CardData;
use crate::ui::components::Card;

#[component]
pub fn AnimationCard(card: CardData, facedown: bool) -> Element {
    rsx!(Card {
        card,
        class: "w-full",
        is_selected: false,
        show_highlight_on_select: false,
        show_dotted_highlight: false,
        show_blue_aura: false,
        show_orange_aura: false,
        facedown,
        use_extra_deck_back: false,
        onclick: |_| {},
    })
}
