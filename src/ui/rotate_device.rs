use dioxus::prelude::*;

use super::components::svg::RotateDeviceIcon;

#[component]
pub fn RotateDeviceOverlay() -> Element {
    rsx!(
        div {
            class: "fixed inset-0 z-50 flex flex-col items-center justify-center bg-gray-800 p-6 text-center hidden portrait:flex",
            div {
                class: "w-16 h-16 mb-4 text-white",
                RotateDeviceIcon { }
            }
            h2 {
                class: "text-xl font-bold text-white mb-2",
                "Please rotate your device"
            }
            p {
                class: "text-slate-400 text-sm max-w-xs",
                "Project Tensho is designed to be played in landscape mode."
            }
        }
    )
}
