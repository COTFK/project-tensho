mod deck;
mod labels;
mod script;

pub use deck::EXTRA_DECK_IDS;
pub use deck::MAIN_DECK_IDS;
pub use deck::STATIC_CARD_DATA;
pub use labels::cache_labels;
pub use labels::get_cached_label;
pub use script::cache_scripts;
pub use script::get_cached_script;

use dioxus::prelude::*;

pub static CARD_BACK: Asset = asset!("/assets/images/cover.png");
pub static EXTRA_BACK: Asset = asset!("/assets/images/cover_extra.png");

pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(hash) => hash,
    None => "unknown",
};

pub async fn cache_card_data() {
    let all_cards = MAIN_DECK_IDS
        .into_iter()
        .chain(EXTRA_DECK_IDS)
        .collect::<Vec<_>>();

    cache_scripts(&all_cards).await;
    cache_labels(&all_cards).await;
}
