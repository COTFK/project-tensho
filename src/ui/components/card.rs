use dioxus::prelude::*;

#[component]
pub fn Card(
    code: u32,
    class: String,
    is_selected: bool,
    show_highlight_on_select: bool,
    show_dotted_highlight: bool,
    show_blue_aura: bool,
    show_orange_aura: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: "relative {class}",
            class: if show_highlight_on_select {"border-2"},
            class: if show_dotted_highlight && !is_selected {"border-yellow-300 border-dashed"},
            class: if is_selected {"border-yellow-300"},
            class: if !is_selected && !show_dotted_highlight {"border-transparent"},
            onclick: onclick,
            div {
                class: "absolute -inset-[5px] rounded-[4px] blur-[2px] mix-blend-screen pointer-events-none",
                class: if show_orange_aura { "bg-yellow-400"},
                class: if show_blue_aura && !show_orange_aura {"bg-cyan-400"},
                class: if !show_blue_aura && !show_orange_aura {"hidden"}
            }
            img {
                class: "relative w-full",
                image_rendering: "smooth",
                aspect_ratio: "59/86",
                src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", code),
            }
            div {
                class: "absolute inset-0 border-5 blur-[2px] mix-blend-screen pointer-events-none animate-pulse",
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
            class: "{class} flex flex-row gap-4 bg-black/60 items-center justify-center px-8 py-2",
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
