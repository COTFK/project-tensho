mod deck;
mod labels;
pub mod macros;
mod script;
mod courtier;
mod card_database;

pub use deck::EXTRA_DECK_IDS;
pub use deck::MAIN_DECK_IDS;
pub use card_database::get_card_data;
pub use card_database::store_card_data;
pub use labels::cache_labels;
pub use labels::get_cached_label;
pub use labels::get_optional_string_label;
pub use script::cache_scripts;
pub use script::get_cached_script;
pub use courtier::fetch_card_data;

use dioxus::prelude::*;

pub static CARD_BACK: Asset = asset!("/assets/images/cover.png");
pub static EXTRA_BACK: Asset = asset!("/assets/images/cover_extra.png");

pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(hash) => hash,
    None => "unknown",
};
pub const COURTIER_API_URL: &str = env!("COURTIER_API_URL");
