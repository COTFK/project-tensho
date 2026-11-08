use super::{COURTIER_API_URL, courtier_card_code_batches, courtier_card_code_query};
use anyhow::Context;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use tracing::warn;

pub type Scripts = Rc<HashMap<String, Vec<u8>>>;

async fn fetch_system_script_names(client: &reqwest::Client) -> anyhow::Result<Vec<String>> {
    let url = format!("{COURTIER_API_URL}/v1/scripts/system/files");

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

async fn fetch_system_script(
    client: &reqwest::Client,
    name: &str,
) -> anyhow::Result<(String, Vec<u8>)> {
    let url = format!("{COURTIER_API_URL}/v1/scripts/system/{name}");
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
    card_codes: &[u32],
) -> anyhow::Result<HashMap<String, Option<String>>> {
    let url = format!("{COURTIER_API_URL}/v1/scripts/cards");
    let mut scripts = HashMap::new();
    for batch in courtier_card_code_batches(card_codes) {
        let response = client
            .get(&url)
            .query(&courtier_card_code_query(batch))
            .send()
            .await?
            .error_for_status()
            .context("Unable to fetch card scripts")?
            .json::<HashMap<String, Option<String>>>()
            .await?;
        scripts.extend(response);
    }

    Ok(scripts)
}

pub async fn cache_scripts(deck_card_codes: &[u32]) -> anyhow::Result<Scripts> {
    let client = reqwest::Client::new();
    let unique_codes: Vec<u32> = deck_card_codes
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (system_names, card_scripts) = futures::try_join!(
        fetch_system_script_names(&client),
        fetch_card_scripts(&client, &unique_codes),
    )?;
    let base_scripts = try_join_all(
        system_names
            .iter()
            .map(|name| fetch_system_script(&client, name)),
    )
    .await?;

    let mut scripts: HashMap<String, Vec<u8>> = base_scripts.into_iter().collect();
    insert_card_scripts(&mut scripts, &unique_codes, card_scripts)?;

    Ok(Rc::new(scripts))
}

fn insert_card_scripts(
    scripts: &mut HashMap<String, Vec<u8>>,
    codes: &[u32],
    mut card_scripts: HashMap<String, Option<String>>,
) -> anyhow::Result<()> {
    for code in codes {
        match card_scripts.remove(&format!("{code:08}")) {
            Some(Some(lua)) => {
                scripts.insert(format!("c{code}.lua"), lua.into_bytes());
            }
            Some(None) => warn!("No card script available for {code:08}"),
            None => anyhow::bail!("Courtier omitted card script entry for {code:08}"),
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/utility/script.rs"]
mod tests;
