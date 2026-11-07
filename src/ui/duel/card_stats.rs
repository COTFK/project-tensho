use dioxus::prelude::*;

use crate::ocgcore::{CardData, CardType};
use crate::utility::get_card_data;

#[component]
pub(super) fn CardStats(card: CardData) -> Element {
    let original_card = get_card_data(card.card_code);
    rsx! {
        div {
            class: "absolute top-0 left-0 text-white text-[8px] md:text-xs lg:text-base font-semibold z-5 bg-gray-900/75 pl-0.5 pr-1 pb-0.5 rounded-br-lg",
            class: if !card.card_type.contains(CardType::MONSTER) {"hidden"},
            if card.level.unwrap_or(0) > 0 || card.rank.unwrap_or(0) > 0 {
                p {
                    span { "✪ " }
                    if card.level.unwrap_or(0) > 0 {
                        span {
                            class: if card.level.unwrap_or(0) > original_card.level {"text-green-400"},
                            class: if card.level.unwrap_or(0) < original_card.level {"text-red-400"},
                            {card.level.unwrap_or(0).to_string()}
                        }
                    }
                    if card.rank.unwrap_or(0) > 0 {
                        span {
                            class: if card.rank.unwrap_or(0) > original_card.level {"text-green-400"},
                            class: if card.rank.unwrap_or(0) < original_card.level {"text-red-400"},
                            {card.rank.unwrap_or(0).to_string()}
                        }
                    }
                }
            }
            if card.link_rating.unwrap_or(0) > 0 {
                p {
                    span { "L-" }
                    span {
                        class: if card.link_rating.unwrap_or(0) > original_card.level {"text-green-400"},
                        class: if card.link_rating.unwrap_or(0) < original_card.level {"text-red-400"},
                        {card.link_rating.unwrap_or(0).to_string()}
                    }
                }
            }
        }
        div {
            class: "absolute bottom-0 right-0 text-white text-[8px] md:text-xs lg:text-base font-semibold z-5 bg-gray-900/75 pr-0.5 pl-1 pt-0.5 rounded-tl-lg",
            class: if !card.card_type.contains(CardType::MONSTER) {"hidden"},
            p {
                span {
                    class: if card.attack.unwrap_or(0) as i32 > original_card.attack {"text-green-400"},
                    class: if (card.attack.unwrap_or(0) as i32) < original_card.attack {"text-red-400"},
                    {card.attack.unwrap_or(0).to_string()}
                }
                span {
                    class: if card.card_type.contains(CardType::LINK) {"hidden"},
                    " / "
                }
                span {
                    class: if card.card_type.contains(CardType::LINK) {"hidden"},
                    class: if card.defense.unwrap_or(0) as i32 > original_card.defense {"text-green-400"},
                    class: if (card.defense.unwrap_or(0) as i32) < original_card.defense {"text-red-400"},
                    {card.defense.unwrap_or(0).to_string()}
                }
            }
        }
    }
}
