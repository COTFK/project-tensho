use anyhow::anyhow;
use dioxus::prelude::*;
use futures::future::join_all;
use std::collections::HashMap;
use std::collections::HashSet;

static SCRIPT_FOLDER: Asset = asset!("/assets/scripts");
static BASE_SCRIPTS: [&str; 25] = [
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
    let path = format!("{SCRIPT_FOLDER}/{name}");

    dioxus::asset_resolver::read_asset_bytes(path)
        .await
        .map_err(|e| anyhow!("Unable to load script bytes: {e}"))
}

pub async fn cache_scripts(deck_card_ids: &[u32]) {
    let mut base_tasks = Vec::new();
    for script in BASE_SCRIPTS {
        base_tasks.push(async move {
            match get_script_data(script).await {
                Ok(bytes) if !bytes.is_empty() => Some((script.to_string(), bytes)),
                _ => {
                    warn!("Failed to load baseline script: {script}");
                    None
                }
            }
        });
    }

    let mut seen = HashSet::new();
    let mut card_tasks = Vec::new();

    for &id in deck_card_ids {
        if seen.insert(id) {
            let script_name = format!("c{id}.lua");
            card_tasks.push(async move {
                match get_script_data(&script_name).await {
                    Ok(bytes) if !bytes.is_empty() => Some((script_name, bytes)),
                    _ => {
                        warn!("Failed to load card script: {script_name}");
                        None
                    }
                }
            });
        }
    }

    let (base_results, card_results) = futures::join!(join_all(base_tasks), join_all(card_tasks));

    for item in base_results
        .into_iter()
        .flatten()
        .chain(card_results.into_iter().flatten())
    {
        cache_script(&item.0, item.1);
    }
}
