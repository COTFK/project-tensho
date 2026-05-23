mod deck;
mod labels;
mod script;

pub use labels::CardLabel;
pub use labels::cache_labels;
pub use labels::get_cached_label;
pub use script::cache_scripts;
pub use script::get_cached_script;

pub use deck::EXTRA_DECK_IDS;
pub use deck::MAIN_DECK_IDS;
pub use deck::STATIC_CARD_DATA;

use dioxus::prelude::*;

pub static CARD_BACK: Asset = asset!("/assets/images/cover.png");