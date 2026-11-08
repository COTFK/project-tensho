use super::*;
use dioxus::core::{AttributeValue, Mutation};
use std::cell::RefCell;

thread_local! {
    static TRIGGER: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
    static SUPPRESSED: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
    static CARD: RefCell<Option<Signal<Option<u32>>>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let trigger = use_signal(|| false);
    let suppressed = use_signal(|| false);
    TRIGGER.with(|slot| *slot.borrow_mut() = Some(trigger));
    SUPPRESSED.with(|slot| *slot.borrow_mut() = Some(suppressed));
    let suppressed = use_memo(move || suppressed());
    use_context_provider(|| ModalSuppression(suppressed));
    rsx! {
        MessageModal { title: "Message", trigger: trigger(), button { "Action" } }
        PickerModal { title: "Picker", trigger: trigger(), button { "Action" } }
    }
}

fn modal_classes(edits: &[Mutation], closed: bool) -> Vec<dioxus::core::ElementId> {
    edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "class",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value.contains("transition-all")
                && value.contains("-translate-y-[calc(100%+2vh+3rem)]") == closed =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn both_modal_types_slide_and_remain_mounted_when_suppressed() {
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let modal_ids = modal_classes(&initial.edits, true);
    assert_eq!(modal_ids.len(), 2, "{:?}", initial.edits);

    TRIGGER.with(|slot| slot.borrow().unwrap().set(true));
    let opened = dom.render_immediate_to_vec();
    assert_eq!(modal_classes(&opened.edits, false), modal_ids);
    assert_eq!(
        opened
            .edits
            .iter()
            .filter(|edit| matches!(edit,
                Mutation::SetAttribute { name: "inert", value: AttributeValue::None, id, .. }
                    if modal_ids.contains(id)
            ))
            .count(),
        2
    );

    SUPPRESSED.with(|slot| slot.borrow().unwrap().set(true));
    let blocked = dom.render_immediate_to_vec();
    assert_eq!(modal_classes(&blocked.edits, true), modal_ids);
    assert_eq!(
        blocked
            .edits
            .iter()
            .filter(|edit| matches!(edit,
                Mutation::SetAttribute { name: "inert", value: AttributeValue::Text(_), id, .. }
                    if modal_ids.contains(id)
            ))
            .count(),
        2
    );
    assert!(!blocked.edits.iter().any(|edit| matches!(edit,
        Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if modal_ids.contains(id)
    )));

    SUPPRESSED.with(|slot| slot.borrow().unwrap().set(false));
    let unblocked = dom.render_immediate_to_vec();
    assert_eq!(modal_classes(&unblocked.edits, false), modal_ids);

    TRIGGER.with(|slot| slot.borrow().unwrap().set(false));
    let closed = dom.render_immediate_to_vec();
    assert_eq!(modal_classes(&closed.edits, true), modal_ids);

    TRIGGER.with(|slot| *slot.borrow_mut() = None);
    SUPPRESSED.with(|slot| *slot.borrow_mut() = None);
}

fn card_picker_app() -> Element {
    let card = use_signal(|| None::<u32>);
    CARD.with(|slot| *slot.borrow_mut() = Some(card));
    let current = card();
    rsx! {
        PickerModal {
            title: if current.is_some() { "Select a card" } else { "Empty" },
            trigger: current.is_some(),
            retain_on_close: true,
            if let Some(code) = current {
                div {
                    img { src: "{code}" }
                }
            }
        }
    }
}

fn image_id(edits: &[Mutation], code: &str) -> dioxus::core::ElementId {
    edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "src",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value == code => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing image {code}: {edits:?}"))
}

struct TestTransition;

impl dioxus::html::HasTransitionData for TestTransition {
    fn property_name(&self) -> String {
        "translate".into()
    }
    fn pseudo_element(&self) -> String {
        String::new()
    }
    fn elapsed_time(&self) -> f32 {
        0.3
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct TestEventConverter;

macro_rules! unused_event_converters {
    ($($method:ident => $data:ident),* $(,)?) => {
        $(
            fn $method(&self, _: &dioxus::html::PlatformEventData) -> dioxus::html::$data {
                panic!("Unexpected event conversion: {}", stringify!($method))
            }
        )*
    };
}

impl dioxus::html::HtmlEventConverter for TestEventConverter {
    fn convert_transition_data(
        &self,
        _: &dioxus::html::PlatformEventData,
    ) -> dioxus::html::TransitionData {
        dioxus::html::TransitionData::new(TestTransition)
    }

    unused_event_converters! {
        convert_animation_data => AnimationData,
        convert_cancel_data => CancelData,
        convert_clipboard_data => ClipboardData,
        convert_composition_data => CompositionData,
        convert_drag_data => DragData,
        convert_focus_data => FocusData,
        convert_form_data => FormData,
        convert_image_data => ImageData,
        convert_keyboard_data => KeyboardData,
        convert_media_data => MediaData,
        convert_mounted_data => MountedData,
        convert_mouse_data => MouseData,
        convert_pointer_data => PointerData,
        convert_resize_data => ResizeData,
        convert_scroll_data => ScrollData,
        convert_selection_data => SelectionData,
        convert_toggle_data => ToggleData,
        convert_touch_data => TouchData,
        convert_visible_data => VisibleData,
        convert_wheel_data => WheelData,
    }
}

fn transition_end(dom: &VirtualDom, id: dioxus::core::ElementId) {
    let data = dioxus::html::PlatformEventData::new(Box::new(()));
    dom.runtime().handle_event(
        "transitionend",
        dioxus::core::Event::new(std::rc::Rc::new(data), true).into_any(),
        id,
    );
}

#[test]
fn card_list_is_retained_until_its_own_transition_ends() {
    dioxus::html::set_event_converter(Box::new(TestEventConverter));
    let mut dom = VirtualDom::new(card_picker_app);
    let initial = dom.rebuild_to_vec();
    let modal_id = modal_classes(&initial.edits, true)[0];
    let content_id = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::NewEventListener { name, id }
                if name == "transitionend" && *id != modal_id =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing content transition listener: {:?}", initial.edits));
    dom.process_events();

    CARD.with(|slot| slot.borrow().unwrap().set(Some(42)));
    let opened = dom.render_immediate_to_vec();
    let first_image = image_id(&opened.edits, "42");
    dom.process_events();

    CARD.with(|slot| slot.borrow().unwrap().set(None));
    let closing = dom.render_immediate_to_vec();
    assert!(
        !closing.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == first_image
        )),
        "{:?}",
        closing.edits
    );
    dom.process_events();

    transition_end(&dom, content_id);
    let child_finished = dom.render_immediate_to_vec();
    assert!(
        !child_finished
            .edits
            .iter()
            .any(|edit| matches!(edit, Mutation::Remove { .. } | Mutation::ReplaceWith { .. })),
        "{:?}",
        child_finished.edits
    );

    CARD.with(|slot| slot.borrow().unwrap().set(Some(100)));
    let reopened = dom.render_immediate_to_vec();
    let second_image = image_id(&reopened.edits, "100");
    dom.process_events();
    transition_end(&dom, modal_id);
    let after_old_transition = dom.render_immediate_to_vec();
    assert!(
        !after_old_transition.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == second_image
        )),
        "{:?}",
        after_old_transition.edits
    );

    CARD.with(|slot| slot.borrow().unwrap().set(None));
    let closing_again = dom.render_immediate_to_vec();
    assert!(
        !closing_again.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == second_image
        )),
        "{:?}",
        closing_again.edits
    );
    transition_end(&dom, modal_id);
    let retracted = dom.render_immediate_to_vec();
    assert!(
        retracted
            .edits
            .iter()
            .any(|edit| matches!(edit, Mutation::Remove { .. } | Mutation::ReplaceWith { .. })),
        "{:?}",
        retracted.edits
    );
    CARD.with(|slot| *slot.borrow_mut() = None);
}
