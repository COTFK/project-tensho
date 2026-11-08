use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RetainedContent {
    visible: bool,
    content: Option<(String, Element)>,
}

#[derive(Clone, Copy)]
pub(crate) struct ModalSuppression(pub Memo<bool>);

#[component]
pub fn MessageModal(title: String, trigger: bool, children: Element) -> Element {
    let suppressed =
        try_consume_context::<ModalSuppression>().is_some_and(|suppression| (suppression.0)());
    let visible = trigger && !suppressed;
    rsx!(
        div {
            class: "absolute top-[2vh] left-1/2 -translate-x-[50%] z-40",
            class: "w-max max-w-[80vw] py-2 px-4",
            class: "flex items-center justify-between gap-4 rounded-lg",
            class: "bg-gray-700/80 shadow-xl transition-all duration-300 ease-in-out text-sm md:text-lg",
            class: if !visible { "-translate-y-[calc(100%+2vh+3rem)]" },
            inert: if !visible { Some("") } else { None },
            p {
                class: "text-white font-semibold text-gray-300 w-max",
                "{title}"
            }
            {children}
        }
    )
}

#[component]
pub fn PickerModal(
    title: String,
    trigger: bool,
    #[props(default)] retain_on_close: bool,
    children: Element,
) -> Element {
    let suppressed =
        try_consume_context::<ModalSuppression>().is_some_and(|suppression| (suppression.0)());
    let visible = trigger && !suppressed;
    let retained = use_hook(|| Rc::new(RefCell::new(RetainedContent::default())));
    let mut revision = use_signal(|| 0u64);
    let _ = revision();
    let (title, children) = {
        let mut retained = retained.borrow_mut();
        retained.visible = visible;
        if visible && retain_on_close {
            retained.content = Some((title.clone(), children.clone()));
        }
        if retain_on_close && !visible {
            retained.content.clone().unwrap_or((title, children))
        } else {
            (title, children)
        }
    };
    rsx!(
        div {
            class: "absolute top-[2vh] left-1/2 -translate-x-[50%] z-40 max-w-[80vw] min-w-[50vw] w-max max-h-[75vh] p-4",
            class: "flex flex-col items-center justify-center gap-4",
            class: "bg-gray-700/80 shadow-xl transition-all duration-300 ease-in-out text-sm rounded-lg",
            class: if !visible { "-translate-y-[calc(100%+2vh+3rem)]" },
            inert: if !visible { Some("") } else { None },
            ontransitionend: move |_| {
                let mut retained = retained.borrow_mut();
                if !retained.visible && retained.content.take().is_some() {
                    revision += 1;
                }
            },
            p {
                class: "text-white font-semibold text-gray-300 w-max md:text-lg",
                "{title}"
            }
            div {
                class: "contents",
                ontransitionend: move |event| event.stop_propagation(),
                {children}
            }
        }
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/ui/modals.rs"]
mod tests;
