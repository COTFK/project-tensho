use anyhow::anyhow;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;

static CARD_DATA_FOLDER: Asset = asset!("/assets/card_data");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLabel {
    pub name: String,
    pub optional_strings: HashMap<usize, String>
}

thread_local! {
    static LABEL_CACHE: std::cell::RefCell<HashMap<String, CardLabel>> = std::cell::RefCell::new(HashMap::new());
}

fn cache_label(id: String, data: CardLabel) {
    LABEL_CACHE.with(|cache| {
        cache.borrow_mut().insert(id, data);
    });
}

pub fn get_cached_label(id: u32) -> Option<CardLabel> {
    let padded_id = format!("{:08}", id);
    LABEL_CACHE.with(|cache| cache.borrow().get(padded_id.as_str()).cloned())
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

    let mut optional_strings = HashMap::new();

    if id == "44455560" { // Ulcanix
        optional_strings.insert(2, String::from("Make this card's Level become that added monster's?"));
    }
    if id == "65305978" {
        optional_strings.insert(0, String::from("Place 1 \"Fire King Island\" from your Deck face-up in your Field Zone?"));
    }
    if id == "57554544" {
        optional_strings.insert(0, String::from("Destroy 1 monster in your hand/field, and search 1 \"Fire King\" monster"));
        optional_strings.insert(1, String::from("Special Summon 1 FIRE Winged Beast from your hand"));
    }
    if id == "02526224" {
        optional_strings.insert(2, String::from("Destroy 1 card on the field?"));
    }

    Ok(CardLabel { name, optional_strings })
}

pub async fn cache_labels(deck_card_ids: &[u32]) {
    let mut seen = HashSet::new();

    for &id in deck_card_ids {
        if seen.insert(id) {
            let padded_id = format!("{:08}", id);
            if let Ok(data) = get_label_data(&padded_id).await {
                cache_label(padded_id, data);
            }
        }
    }
}
