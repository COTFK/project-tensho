use dioxus::prelude::*;

#[component]
pub(super) fn ChainBadges(links: Vec<u32>, #[props(default)] rotation: f64) -> Element {
    rsx! {
        if !links.is_empty() {
            div {
                class: "absolute top-0 right-0 z-10 flex flex-col items-end gap-0.5 pointer-events-none [transform:rotate(var(--chain-rotation))]",
                style: "--chain-rotation: {-rotation}deg;",
                for number in links {
                    span {
                        key: "{number}",
                        class: "flex size-6 items-center justify-center rounded-full bg-slate-300/66 text-xs font-bold text-black ring-2 ring-black md:size-8 md:text-base",
                        role: "img",
                        "aria-label": "Chain link {number}; higher links resolve first",
                        title: "Chain link {number}",
                        "{number}"
                    }
                }
            }
        }
    }
}
