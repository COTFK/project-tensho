use dioxus::prelude::*;

#[component]
pub fn SelectableCard(card_code: u32, select_signal: WriteSignal<Option<u8>>, value_to_set: u8) -> Element {
    rsx!(
        img {
            class: "relative w-[12vw] border-2 z-20",
            class: if select_signal() == Some(value_to_set) { "border-yellow-300" } else { "border-transparent" },
            src: format!("https://images.ygoprodeck.com/images/cards/{}.jpg", card_code),
            onclick: move |_| {
                select_signal.set(Some(value_to_set));
            }
        }
    )
}