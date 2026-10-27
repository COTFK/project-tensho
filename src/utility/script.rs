use super::{COURTIER_API_URL, courtier_card_id_batches, courtier_card_id_query};
use anyhow::Context;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use tracing::warn;

pub type Scripts = Rc<HashMap<String, Vec<u8>>>;

async fn fetch_system_scripts(client: &reqwest::Client) -> anyhow::Result<Vec<String>> {
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

async fn fetch_base_script(
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
    card_ids: &[u32],
) -> anyhow::Result<HashMap<String, Option<String>>> {
    let url = format!("{COURTIER_API_URL}/v1/scripts/cards");
    let mut scripts = HashMap::new();
    for batch in courtier_card_id_batches(card_ids) {
        let response = client
            .get(&url)
            .query(&courtier_card_id_query(batch))
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

pub async fn cache_scripts(deck_card_ids: &[u32]) -> anyhow::Result<Scripts> {
    let client = reqwest::Client::new();
    let unique_ids: Vec<u32> = deck_card_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (system_names, card_scripts) = futures::try_join!(
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
    insert_card_scripts(&mut scripts, &unique_ids, card_scripts)?;

    Ok(Rc::new(scripts))
}

fn insert_card_scripts(
    scripts: &mut HashMap<String, Vec<u8>>,
    ids: &[u32],
    mut card_scripts: HashMap<String, Option<String>>,
) -> anyhow::Result<()> {
    for id in ids {
        match card_scripts.remove(&format!("{id:08}")) {
            Some(Some(lua)) => {
                scripts.insert(format!("c{id}.lua"), lua.into_bytes());
            }
            Some(None) => warn!("No card script available for {id:08}"),
            None => anyhow::bail!("Courtier omitted card script entry for {id:08}"),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_scripts_allow_null_but_not_omitted_entries() {
        let response = r#"{"02526224":"lua source","57554544":null}"#;
        let card_scripts = serde_json::from_str(response).unwrap();
        let mut scripts = HashMap::new();

        insert_card_scripts(&mut scripts, &[2526224, 57554544], card_scripts).unwrap();
        assert_eq!(scripts.get("c2526224.lua").unwrap(), b"lua source");
        assert_eq!(scripts.len(), 1);
        assert!(insert_card_scripts(&mut scripts, &[1], HashMap::new()).is_err());
    }
}
