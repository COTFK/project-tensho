use super::COURTIER_API_URL;
use anyhow::Context;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use tracing::warn;

pub type Scripts = Rc<HashMap<String, Vec<u8>>>;

static BASE_SCRIPTS: [&str; 26] = [
    "constant.lua",
    "utility.lua",
    "card_counter_constants.lua",
    "archetype_setcode_constants.lua",
    "chain.lua",
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

async fn fetch_base_script(
    client: &reqwest::Client,
    name: &str,
) -> anyhow::Result<(String, Vec<u8>)> {
    let url = format!("{COURTIER_API_URL}/script/{name}");
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("Unable to fetch script {name}"))?
        .bytes()
        .await?;

    Ok((name.to_string(), bytes.to_vec()))
}

async fn fetch_card_scripts(
    client: &reqwest::Client,
    card_ids: &[u32],
) -> anyhow::Result<HashMap<String, String>> {
    let url = format!("{COURTIER_API_URL}/card/script");
    let body = HashMap::from([("card_ids", card_ids)]);

    let scripts = client
        .post(url)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .context("Unable to fetch card scripts")?
        .json()
        .await?;

    Ok(scripts)
}

pub async fn cache_scripts(deck_card_ids: &[u32]) -> anyhow::Result<Scripts> {
    let client = reqwest::Client::new();
    let unique_ids: Vec<u32> = deck_card_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (base_scripts, mut card_scripts) = futures::try_join!(
        try_join_all(
            BASE_SCRIPTS
                .iter()
                .map(|name| fetch_base_script(&client, name))
        ),
        fetch_card_scripts(&client, &unique_ids),
    )?;

    let mut scripts: HashMap<String, Vec<u8>> = base_scripts.into_iter().collect();
    for id in unique_ids {
        match card_scripts.remove(&format!("{id:08}")) {
            Some(lua) => {
                scripts.insert(format!("c{id}.lua"), lua.into_bytes());
            }
            None => warn!("No card script available for {id:08}"),
        }
    }

    Ok(Rc::new(scripts))
}
