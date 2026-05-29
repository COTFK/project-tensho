use anyhow::anyhow;
use dioxus::prelude::*;
use futures::future::join_all;
use std::collections::HashMap;
use std::collections::HashSet;

static CARD_DATA_FOLDER: Asset = asset!("/assets/card_data");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLabel {
    pub name: String,
    pub optional_strings: HashMap<usize, String>,
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
    LABEL_CACHE.with(|cache| cache.borrow().get(&padded_id).cloned())
}

async fn get_label_data(id: &str) -> anyhow::Result<CardLabel> {
    let path = format!("{CARD_DATA_FOLDER}/{id}.json");
    let bytes = dioxus::asset_resolver::read_asset_bytes(path)
        .await
        .map_err(|e| anyhow!("Unable to load card data: {e}"))?;

    let json: serde_json::Value = serde_json::from_slice(&bytes)?;

    let name = json["name"]["en"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing name.en in response"))?
        .to_string();

    let mut optional_strings = HashMap::new();

    match id {
        "44455560" => {
            // Ulcanix
            optional_strings.insert(
                2,
                "Make this card's Level become that added monster's?".to_string(),
            );
        }
        "65305978" => {
            optional_strings.insert(
                0,
                "Place 1 \"Fire King Island\" from your Deck face-up in your Field Zone?"
                    .to_string(),
            );
        }
        "57554544" => {
            // Fire King Island
            optional_strings.insert(
                0,
                "Destroy 1 monster in your hand/field, and search 1 \"Fire King\" monster"
                    .to_string(),
            );
            optional_strings.insert(
                1,
                "Special Summon 1 FIRE Winged Beast from your hand".to_string(),
            );
        }
        "02526224" => {
            // Arvata
            optional_strings.insert(2, "Destroy 1 card on the field?".to_string());
        }
        "49238328" => {
            // Extravagance
            optional_strings.insert(0, "Banish 3 cards".to_string());
            optional_strings.insert(1, "Banish 6 cards".to_string());
        }
        _ => {}
    }

    Ok(CardLabel {
        name,
        optional_strings,
    })
}

pub async fn cache_labels(deck_card_ids: &[u32]) {
    let mut seen = HashSet::new();
    let mut tasks = Vec::new();

    for &id in deck_card_ids {
        if seen.insert(id) {
            let padded_id = format!("{:08}", id);
            tasks.push(async move {
                match get_label_data(&padded_id).await {
                    Ok(data) => Some((padded_id, data)),
                    Err(_) => {
                        warn!("Failed to fetch label data for card ID: {padded_id}");
                        None
                    }
                }
            });
        }
    }

    let results = join_all(tasks).await;

    for item in results.into_iter().flatten() {
        cache_label(item.0, item.1);
    }
}

pub fn get_optional_string_label(card_code: u32, string_index: usize) -> String {
    get_cached_label(card_code)
        .and_then(|card_label| card_label.optional_strings.get(&string_index).cloned())
        .unwrap_or_else(|| String::from("error"))
}
