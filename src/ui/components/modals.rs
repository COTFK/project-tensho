use dioxus::prelude::*;

#[component]
pub fn MessageModal(title: String, trigger: bool, children: Element) -> Element {
    rsx!(
        div {
            class: "absolute left-1/2 -translate-x-[50%] z-40",
            class: "w-max max-w-[90vw] py-2 px-4",
            class: "flex items-center justify-between gap-4 rounded-lg",
            class: "bg-gray-700/80 shadow-xl transition-all duration-300 ease-in-out text-sm",
            class: if trigger { "top-[2vh]" } else { "-top-[25%]" },
            p {
                class: "text-white font-semibold text-gray-300 w-max",
                "{title}"
            }
            {children}
        }
    )
}

#[component]
pub fn CardPickerModal(title: String, trigger: bool, children: Element) -> Element {
    rsx!(
        div {
            class: "absolute left-1/2 -translate-x-[50%] z-40 max-w-[60vw] min-w-[50vw] max-h-[75vh] p-4",
            class: "flex flex-col items-center justify-center gap-4",
            class: "bg-gray-700/80 shadow-xl transition-all duration-300 ease-in-out text-sm rounded-lg",
            class: if trigger { "top-[2vh]" } else { "-top-[75vh]" },
            p {
                class: "text-white font-semibold text-gray-300 w-max",
                "{title}"
            }
            {children}
        }
    )
}