mod card_catalog;
mod card_text;
mod courtier;
mod deck;
pub mod macros;
mod script;

pub use card_catalog::get_card_text;
pub use card_catalog::read_card_definition;
pub use card_catalog::store_card_definitions;
pub use card_text::get_optional_string_label;
pub use courtier::ARCHETYPE_SCRIPT;
pub use courtier::fetch_card_definitions;
pub use deck::EXTRA_DECK_CODES;
pub use deck::MAIN_DECK_CODES;
pub use script::Scripts;
pub use script::cache_scripts;

use dioxus::prelude::*;

pub static CARD_BACK: Asset = asset!("/assets/images/cover.png");
pub static EXTRA_BACK: Asset = asset!("/assets/images/cover_extra.png");

pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(hash) => hash,
    None => "unknown",
};
pub const COURTIER_API_URL: &str = env!("COURTIER_API_URL");

const COURTIER_CARD_BATCH_SIZE: usize = 100;

fn courtier_card_code_batches(codes: &[u32]) -> std::slice::Chunks<'_, u32> {
    codes.chunks(COURTIER_CARD_BATCH_SIZE)
}

fn courtier_card_code_query(codes: &[u32]) -> Vec<(&'static str, String)> {
    codes
        .iter()
        .map(|code| ("ids", format!("{code:08}")))
        .collect()
}

pub fn card_image_url(code: u32) -> String {
    format!("{COURTIER_API_URL}/v1/images/{code:08}")
}

pub fn card_thumbnail_url(code: u32) -> String {
    format!("{}/thumbnail", card_image_url(code))
}

#[cfg(test)]
#[path = "../../tests/unit/utility/mod.rs"]
mod courtier_api_tests;
