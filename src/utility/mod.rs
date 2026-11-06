mod card_database;
mod courtier;
mod deck;
mod labels;
pub mod macros;
mod script;

pub use card_database::get_card_data;
pub use card_database::get_card_label;
pub use card_database::store_card_data;
pub use courtier::ARCHETYPE_SCRIPT;
pub use courtier::fetch_card_data;
pub use deck::EXTRA_DECK_IDS;
pub use deck::MAIN_DECK_IDS;
pub use labels::get_optional_string_label;
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

fn courtier_card_id_batches(ids: &[u32]) -> std::slice::Chunks<'_, u32> {
    ids.chunks(COURTIER_CARD_BATCH_SIZE)
}

fn courtier_card_id_query(ids: &[u32]) -> Vec<(&'static str, String)> {
    ids.iter().map(|id| ("ids", format!("{id:08}"))).collect()
}

pub fn card_image_url(id: u32) -> String {
    format!("{COURTIER_API_URL}/v1/images/{id:08}")
}

pub fn card_thumbnail_url(id: u32) -> String {
    format!("{}/thumbnail", card_image_url(id))
}

#[cfg(test)]
#[path = "../../tests/unit/utility/mod.rs"]
mod courtier_api_tests;
