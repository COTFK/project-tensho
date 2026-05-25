use super::svg::Spinner;
use dioxus::prelude::*;

#[component]
pub fn LoadingScreen() -> Element {
    rsx!(
        div {
            class: "flex flex-row gap-4 items-center justify-center h-dvh w-dvw bg-gray-800",
                div {
                    class: "size-12",
                    Spinner {}
                }
                p {
                    class: "text-white font-semibold",
                    "Loading..."
                }
        }

    )
}
