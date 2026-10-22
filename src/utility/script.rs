use super::COURTIER_API_URL;
use anyhow::Context;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use tracing::warn;

pub type Scripts = Rc<HashMap<String, Vec<u8>>>;

async fn fetch_system_scripts(client: &reqwest::Client) -> anyhow::Result<Vec<String>> {
    let url = format!("{COURTIER_API_URL}/scripts/system");

    let names = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .context("Unable to fetch system script list")?
        .json()
        .await?;

    Ok(names)
}

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

    let (system_names, mut card_scripts) = futures::try_join!(
        fetch_system_scripts(&client),
        fetch_card_scripts(&client, &unique_ids),
    )?;
    let base_scripts = try_join_all(
        system_names
            .iter()
            .map(|name| fetch_base_script(&client, name)),
    )
    .await?;

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
