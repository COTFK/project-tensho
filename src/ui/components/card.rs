use dioxus::prelude::*;

use crate::ocgcore::CardData;
use crate::utility::CARD_BACK;
use crate::utility::EXTRA_BACK;

#[component]
pub fn Card(
    card: CardData,
    class: String,
    is_selected: bool,
    show_highlight_on_select: bool,
    show_dotted_highlight: bool,
    show_blue_aura: bool,
    show_orange_aura: bool,
    facedown: bool,
    use_extra_deck_back: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: "relative {class}",
            class: if show_highlight_on_select {"border-2"},
            class: if show_dotted_highlight && !is_selected {"border-yellow-300 border-dashed"},
            class: if is_selected && !(show_orange_aura || show_blue_aura) {"border-yellow-300"},
            class: if !is_selected && !show_dotted_highlight {"border-transparent"},
            onclick: onclick,
            div {
                class: "absolute -inset-[2px] md:-inset-[4px] rounded-[2px] blur-[1px] mix-blend-screen pointer-events-none",
                class: if show_orange_aura { "bg-yellow-400"},
                class: if show_blue_aura && !show_orange_aura {"bg-cyan-400"},
                class: if !show_blue_aura && !show_orange_aura {"hidden"}
            }
            img {
                class: "relative w-full",
                loading: "eager",
                decoding: "async",
                image_rendering: "smooth",
                aspect_ratio: "59/86",
                src: if !facedown {
                    format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card.card_code)
                } else {
                    if use_extra_deck_back {
                        EXTRA_BACK.to_string()
                    } else {
                        CARD_BACK.to_string()
                    }
                },
            }
            div {
                class: "absolute inset-0 md:inset-[4px] border-4 blur-[2px] mix-blend-screen pointer-events-none animate-pulse",
                class: if show_orange_aura { "border-yellow-300/50"},
                class: if show_blue_aura && !show_orange_aura {"border-cyan-300/50"},
                class: if !show_blue_aura && !show_orange_aura {"hidden"}
            }
        }
    }
}

#[component]
pub fn CardActionMenu(class: String, trigger: bool, children: Element) -> Element {
    rsx!(
        div {
            class: "{class} flex flex-row gap-4 bg-black/60 items-center justify-center text-xs md:text-lg",
            class: if !trigger {"hidden"},
            class: "[mask-image:linear-gradient(to_right,transparent_0%,black_20%,black_80%,transparent_100%)] [-webkit-mask-image:linear-gradient(to_right,transparent_0%,black_20%,black_80%,transparent_100%)]",
            {children}
        }
    )
}

#[component]
pub fn CardStack(length: usize, image_url: String) -> Element {
    rsx!(
        for index in 1..(length + 1) {
            div {
                class: "absolute inset-[clamp(2px,0.6vw,8px)]",
                img {
                    class: "w-full h-full object-contain",
                    style: "z-index: 10; transform: translate({index as f32 * 0.01}vw, -{index as f32 * 0.01}vh);",
                    image_rendering: "smooth",
                    aspect_ratio: "59/86",
                    src: "{image_url}",
                }
            }
        }
    )
}
