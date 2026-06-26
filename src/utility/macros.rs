#[macro_export]
macro_rules! define_ui_state {
    ($struct_name:ident { $($field:ident : $type:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct $struct_name {
            $( pub $field: dioxus::prelude::Signal<$type>, )*
        }

        impl $struct_name {
            pub fn reset(&mut self) {
                $(
                    self.$field.set(<$type>::default());
                )*
            }
        }
    };
}