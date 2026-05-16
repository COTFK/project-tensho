use anyhow::anyhow;
use dioxus::prelude::*;

// Add this inside your memory or data module
use std::collections::HashMap;

static SCRIPT_FOLDER: Asset = asset!("/assets/scripts");
static BASE_SCRIPTS: [&'static str; 25] = [
    "constant.lua",
    "utility.lua",
    "card_counter_constants.lua",
    "archetype_setcode_constants.lua",
    "debug_utility.lua",
    "cards_specific_functions.lua",
    "proc_fusion.lua",
    "proc_fusion_spell.lua",
    "proc_ritual.lua",
    "proc_synchro.lua",
    "proc_union.lua",
    "proc_xyz.lua",
    "proc_pendulum.lua",
    "proc_link.lua",
    "proc_equip.lua",
    "proc_persistent.lua",
    "proc_workaround.lua",
    "proc_normal.lua",
    "proc_skill.lua",
    "proc_rush.lua",
    "proc_maximum.lua",
    "proc_gemini.lua",
    "proc_spirit.lua",
    "proc_unofficial.lua",
    "deprecated_functions.lua",
];

std::thread_local! {
    static SCRIPT_CACHE: std::cell::RefCell<HashMap<String, Vec<u8>>> = std::cell::RefCell::new(HashMap::new());
}

fn cache_script(name: &str, data: Vec<u8>) {
    SCRIPT_CACHE.with(|cache| {
        cache.borrow_mut().insert(name.to_string(), data);
    });
}

pub fn get_cached_script(name: &str) -> Option<Vec<u8>> {
    SCRIPT_CACHE.with(|cache| cache.borrow().get(name).cloned())
}

async fn get_script_data(name: &str) -> anyhow::Result<Vec<u8>> {
    let path = format!("{}/{name}", SCRIPT_FOLDER);

    dioxus::asset_resolver::read_asset_bytes(path)
        .await
        .map_err(|e| anyhow!("Unable to load script bytes: {e}"))
}

pub async fn cache_scripts(mut deck_card_ids: Vec<u32>) {
    // Add helper scripts
    for script in BASE_SCRIPTS {
        if let Ok(bytes) = get_script_data(script).await {
            if bytes.len() == 0 {
                warn!("Failed to load {script} - got 0 bytes.");
                continue;
            }
            cache_script(script, bytes);
        }
    }

    // Batch fetch all individual card scripts based on the deck lists
    deck_card_ids.sort();
    deck_card_ids.dedup();

    for id in deck_card_ids {
        let script_name = format!("c{}.lua", id);
        if let Ok(bytes) = get_script_data(&script_name).await {
            if bytes.len() == 0 {
                warn!("Failed to load {script_name} - got 0 bytes.");
                continue;
            }
            cache_script(&script_name, bytes);
        }
    }
}
