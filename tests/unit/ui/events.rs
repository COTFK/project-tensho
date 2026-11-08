use dioxus::html::*;
use dioxus::prelude::*;

struct TestTransition;

impl HasTransitionData for TestTransition {
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

struct TestMouse;

impl InteractionLocation for TestMouse {
    fn client_coordinates(&self) -> geometry::ClientPoint {
        geometry::ClientPoint::zero()
    }
    fn page_coordinates(&self) -> geometry::PagePoint {
        geometry::PagePoint::zero()
    }
    fn screen_coordinates(&self) -> geometry::ScreenPoint {
        geometry::ScreenPoint::zero()
    }
}

impl InteractionElementOffset for TestMouse {
    fn element_coordinates(&self) -> geometry::ElementPoint {
        geometry::ElementPoint::zero()
    }
}

impl ModifiersInteraction for TestMouse {
    fn modifiers(&self) -> Modifiers {
        Modifiers::empty()
    }
}

impl PointerInteraction for TestMouse {
    fn held_buttons(&self) -> input_data::MouseButtonSet {
        input_data::MouseButtonSet::default()
    }
    fn trigger_button(&self) -> Option<input_data::MouseButton> {
        Some(input_data::MouseButton::Primary)
    }
}

impl HasMouseData for TestMouse {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct TestEventConverter;

macro_rules! unused_event_converters {
    ($($method:ident => $data:ident),* $(,)?) => {
        $(
            fn $method(&self, _: &PlatformEventData) -> $data {
                panic!("Unexpected event conversion: {}", stringify!($method))
            }
        )*
    };
}

impl HtmlEventConverter for TestEventConverter {
    fn convert_transition_data(&self, _: &PlatformEventData) -> TransitionData {
        TransitionData::new(TestTransition)
    }

    fn convert_mouse_data(&self, _: &PlatformEventData) -> MouseData {
        MouseData::new(TestMouse)
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

pub fn install() {
    set_event_converter(Box::new(TestEventConverter));
}

pub fn dispatch(dom: &VirtualDom, name: &str, id: dioxus::core::ElementId) {
    let data = PlatformEventData::new(Box::new(()));
    dom.runtime().handle_event(
        name,
        dioxus::core::Event::new(std::rc::Rc::new(data), true).into_any(),
        id,
    );
}
