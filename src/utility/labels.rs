use anyhow::anyhow;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;

static CARD_DATA_FOLDER: Asset = asset!("/assets/card_data");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CardLabel {
    pub name: String,
}

thread_local! {
    static LABEL_CACHE: std::cell::RefCell<HashMap<u32, CardLabel>> = std::cell::RefCell::new(HashMap::new());
}

fn cache_label(id: u32, data: CardLabel) {
    LABEL_CACHE.with(|cache| {
        cache.borrow_mut().insert(id, data);
    });
}

pub fn get_cached_label(id: u32) -> Option<CardLabel> {
    LABEL_CACHE.with(|cache| cache.borrow().get(&id).cloned())
}

async fn get_label_data(id: &str) -> anyhow::Result<CardLabel> {
    let path = format!("{CARD_DATA_FOLDER}/{id}.json");
    let bytes = dioxus::asset_resolver::read_asset_bytes(path)
        .await
        .map_err(|e| anyhow!("Unable to load card data: {e}"))?;

    let json: serde_json::Value = serde_json::from_slice(&bytes)?;

    let name = json["name"]["en"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing name.en in response"))?
        .to_string();

    Ok(CardLabel { name })
}

pub async fn cache_labels(deck_card_ids: &[u32]) {
    let mut seen = HashSet::new();

    for &id in deck_card_ids {
        if seen.insert(id) {
            let padded_id = format!("{:08}", id);
            if let Ok(data) = get_label_data(&padded_id).await {
                cache_label(id, data);
            }
        }
    }
}
