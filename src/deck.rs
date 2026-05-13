use std::collections::{BTreeSet, HashSet, VecDeque};
use tracing::{debug, warn};

const CARDSCRIPTS_BASE_URL: &str = "https://raw.githubusercontent.com/ProjectIgnis/CardScripts/master";
const SUPPORT_SCRIPT_ROOTS: [&str; 2] = ["constant.lua", "utility.lua"];

// Fire King deck - main deck (40 cards)
pub const MAIN_DECK: [u32; 40] = [
    66431519, 66431519, 66431519, 23015896, 44455560, 44455560, 44455560, 90681088,
    90681088, 90681088, 18621798, 2526224, 2526224, 2526224, 14558128, 14558128,
    14558128, 97268402, 97268402, 97268402, 24508238, 33854624, 6637331, 65305978,
    65305978, 65305978, 57554544, 91703676, 91703676, 84211599, 49238328, 49238328,
    49238328, 24224830, 10045474, 10045474, 10045474, 40366667, 40366667, 40366667,
];

// Fire King deck - extra deck (15 cards)
pub const EXTRA_DECK: [u32; 15] = [
    94259633, 60303245, 87871125, 2772337, 2772337, 2772337, 48815792, 48815792,
    8264361, 29301450, 29301450, 93039339, 64182380, 64182380, 64182380,
];

#[derive(Clone, Copy)]
struct StaticCard {
    type_: u32,
    atk: i32,
    def: i32,
    level: u32,
    race: u32,
    attribute: u32,
}

const STATIC_CARD_DATA: &[(u32, StaticCard)] = &[
    (2526224, StaticCard { type_: 33, atk: 2400, def: 200, level: 8, race: 16384, attribute: 4 }),
    (2772337, StaticCard { type_: 67108897, atk: 2700, def: 42, level: 3, race: 8, attribute: 4 }),
    (6637331, StaticCard { type_: 33, atk: 2500, def: 2000, level: 6, race: 8192, attribute: 32 }),
    (8264361, StaticCard { type_: 67108897, atk: 1850, def: 5, level: 2, race: 2, attribute: 32 }),
    (10045474, StaticCard { type_: 4, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (14558127, StaticCard { type_: 4129, atk: 0, def: 1800, level: 3, race: 16, attribute: 4 }),
    (18621798, StaticCard { type_: 33, atk: 1800, def: 200, level: 4, race: 32768, attribute: 4 }),
    (23015896, StaticCard { type_: 33, atk: 2700, def: 1700, level: 8, race: 512, attribute: 4 }),
    (24224830, StaticCard { type_: 65538, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (24508238, StaticCard { type_: 33, atk: 100, def: 100, level: 1, race: 512, attribute: 32 }),
    (28332833, StaticCard { type_: 33, atk: 200, def: 200, level: 1, race: 128, attribute: 4 }),
    (29301450, StaticCard { type_: 67108897, atk: 1600, def: 40, level: 2, race: 1, attribute: 32 }),
    (33854624, StaticCard { type_: 33, atk: 2500, def: 2000, level: 6, race: 8192, attribute: 32 }),
    (40366667, StaticCard { type_: 4, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (44455560, StaticCard { type_: 33, atk: 800, def: 2000, level: 4, race: 512, attribute: 4 }),
    (48815792, StaticCard { type_: 67108897, atk: 1850, def: 5, level: 2, race: 2, attribute: 4 }),
    (49238328, StaticCard { type_: 2, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (57554544, StaticCard { type_: 524290, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (60303245, StaticCard { type_: 67108897, atk: 0, def: 4, level: 1, race: 16777216, attribute: 4 }),
    (64182380, StaticCard { type_: 8388641, atk: 3000, def: 2000, level: 8, race: 128, attribute: 4 }),
    (65305978, StaticCard { type_: 131074, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (66431519, StaticCard { type_: 33, atk: 2700, def: 1700, level: 8, race: 512, attribute: 4 }),
    (84211599, StaticCard { type_: 2, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (87871125, StaticCard { type_: 67108897, atk: 1800, def: 130, level: 2, race: 16777216, attribute: 4 }),
    (90681088, StaticCard { type_: 33, atk: 500, def: 200, level: 1, race: 512, attribute: 4 }),
    (91703676, StaticCard { type_: 65538, atk: 0, def: 0, level: 0, race: 0, attribute: 0 }),
    (93039339, StaticCard { type_: 8388641, atk: 2900, def: 2900, level: 12, race: 8, attribute: 32 }),
    (94259633, StaticCard { type_: 67108897, atk: 0, def: 128, level: 1, race: 2, attribute: 32 }),
    (97268402, StaticCard { type_: 4129, atk: 0, def: 0, level: 1, race: 2, attribute: 16 }),
];

pub async fn fetch_and_register_card(id: u32) -> Result<(), String> {
    if let Some(card) = lookup_static_card(id) {
        register_card_from_static(id, card);
        return Ok(());
    }

    Err(format!("No static card data for {id}"))
}

fn lookup_static_card(id: u32) -> Option<StaticCard> {
    STATIC_CARD_DATA
        .iter()
        .find_map(|(card_id, card)| if *card_id == id { Some(*card) } else { None })
}

fn register_card_from_static(id: u32, card: StaticCard) {
    debug!(
        "Registering static card 0x{:08x}: type=0x{:x}, level={}, attr=0x{:x}, race=0x{:x}",
        id, card.type_, card.level, card.attribute, card.race
    );
    // crate::ocgcore::register_card_data(
    //     id,
    //     card.type_,
    //     card.level,
    //     card.attribute,
    //     card.race,
    //     card.atk,
    //     card.def,
    // );
}

pub async fn fetch_and_load_script(duel: u32, id: u32) -> Result<(), String> {
    let script_name = format!("c{}.lua", id);
    let url = format!("{}/official/{}", CARDSCRIPTS_BASE_URL, script_name);

    match gloo_net::http::Request::get(&url).send().await {
        Ok(resp) => {
            if !resp.ok() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let preview: String = body.lines().next().unwrap_or("").chars().take(120).collect();
                warn!(
                    "Script fetch failed for card {}: HTTP {} preview='{}'",
                    id, status, preview
                );
                return Err(format!("Script fetch failed for card {}: HTTP {}", id, status));
            }

            match resp.text().await {
                Ok(content) => {
                    if content.trim().is_empty() {
                        warn!("Script content empty for card {}", id);
                        return Err(format!("Script content empty for card {}", id));
                    }
                    load_card_script_content(duel, id, &content);
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to read script for card {}: {}", id, e);
                    Err(format!("Failed to read script for card {}: {}", id, e))
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch script for card {}: {}", id, e);
            Err(format!("Failed to fetch script for card {}: {}", id, e))
        }
    }
}

fn wrap_card_script_for_manual_load(id: u32, content: &str) -> String {
    // OCG_LoadScript executes code directly, but card scripts expect load_card_script context
    // (self_code, self_table and a c{id} table) when they call GetID().
    format!(
    "self_code={id}\n\
c{id}=c{id} or {{}}\n\
if c{id}.__tostring==nil and Debug and Debug.CardToStringWrapper then c{id}.__tostring=Debug.CardToStringWrapper end\n\
setmetatable(c{id}, Card)\n\
rawset(c{id},\"__index\",c{id})\n\
self_table=c{id}\n\
{content}\n\
self_table=nil\n\
self_code=nil\n",
    id = id,
    content = content
    )
}

fn load_card_script_content(duel: u32, id: u32, content: &str) -> bool {
    let wrapped = wrap_card_script_for_manual_load(id, content);
    let script_name = format!("c{}.lua", id);
    load_named_script_content(duel, &script_name, &wrapped)
}

fn load_named_script_content(duel: u32, script_name: &str, content: &str) -> bool {
    let result = crate::ocgcore::load_script(duel, content, script_name).unwrap_or(0);
    if result != 0 {
        debug!("Loaded script {}", script_name);
        true
    } else {
        let preview: String = content.lines().next().unwrap_or("").chars().take(120).collect();
        warn!(
            "Failed to load script {} into Lua VM (preview='{}')",
            script_name, preview
        );
        false
    }
}

fn normalize_script_name(name: &str, from_require: bool) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if from_require {
        // Lua require("a.b") maps to a/b.lua; require("a/b") maps to a/b.lua.
        if trimmed.ends_with(".lua") {
            return trimmed.to_string();
        }
        let mut normalized = trimmed.replace('.', "/");
        if !normalized.ends_with(".lua") {
            normalized.push_str(".lua");
        }
        return normalized;
    }

    // dofile()/Duel.LoadScript() typically pass file paths directly.
    if trimmed.ends_with(".lua") {
        trimmed.to_string()
    } else {
        format!("{}.lua", trimmed)
    }
}

fn parse_quoted_arg(s: &str, start: usize) -> Option<(String, usize)> {
    let mut i = start;
    while let Some(ch) = s.as_bytes().get(i) {
        if !ch.is_ascii_whitespace() {
            break;
        }
        i += 1;
    }

    let quote = *s.as_bytes().get(i)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    i += 1;

    let mut out = String::new();
    while let Some(&ch) = s.as_bytes().get(i) {
        if ch == quote {
            return Some((out, i + 1));
        }
        out.push(ch as char);
        i += 1;
    }
    None
}

fn collect_calls(content: &str, pattern: &str, from_require: bool, out: &mut HashSet<String>) {
    let mut pos = 0usize;
    while let Some(found) = content[pos..].find(pattern) {
        let start = pos + found + pattern.len();
        if let Some((name, next)) = parse_quoted_arg(content, start) {
            let normalized = normalize_script_name(&name, from_require);
            if !normalized.is_empty() {
                out.insert(normalized);
            }
            pos = next;
        } else {
            pos = start;
        }
    }
}

fn extract_script_dependencies(content: &str) -> Vec<String> {
    let mut deps = HashSet::new();
    collect_calls(content, "Duel.LoadScript(", false, &mut deps);
    collect_calls(content, "dofile(", false, &mut deps);
    collect_calls(content, "require(", true, &mut deps);
    deps.into_iter().collect()
}

async fn fetch_script_by_name(script_name: &str) -> Result<String, String> {
    let candidates = [
        format!("{}/{}", CARDSCRIPTS_BASE_URL, script_name),
        format!("{}/official/{}", CARDSCRIPTS_BASE_URL, script_name),
    ];

    let mut last_err = String::new();
    for url in &candidates {
        match gloo_net::http::Request::get(url).send().await {
            Ok(resp) => {
                if !resp.ok() {
                    last_err = format!("HTTP {} from {}", resp.status(), url);
                    continue;
                }
                let text = resp
                    .text()
                    .await
                    .map_err(|e| format!("Failed reading script body from {}: {}", url, e))?;
                if text.trim().is_empty() {
                    last_err = format!("Empty script body from {}", url);
                    continue;
                }
                return Ok(text);
            }
            Err(e) => {
                last_err = format!("Request failed for {}: {}", url, e);
            }
        }
    }
    Err(format!("Failed to fetch {} ({})", script_name, last_err))
}

pub async fn preload_support_scripts(duel: u32) {
    let mut queue: VecDeque<String> = SUPPORT_SCRIPT_ROOTS
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut visited = HashSet::new();
    let mut loaded_count = 0usize;

    while let Some(script_name) = queue.pop_front() {
        if !visited.insert(script_name.clone()) {
            continue;
        }

        let content = match fetch_script_by_name(&script_name).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Support script fetch failed for {}: {}", script_name, e);
                continue;
            }
        };

        if load_named_script_content(duel, &script_name, &content) {
            loaded_count += 1;
        }

        for dep in extract_script_dependencies(&content) {
            if !visited.contains(&dep) {
                queue.push_back(dep);
            }
        }
    }

    debug!(
        "Support script preload finished: loaded {} scripts, discovered {} total",
        loaded_count,
        visited.len()
    );
}

pub async fn initialize_duel() -> anyhow::Result<u32> {
    // Collect unique card IDs from main and extra deck
    let mut unique_ids: BTreeSet<u32> = BTreeSet::new();
    for id in MAIN_DECK.iter() {
        unique_ids.insert(*id);
    }
    for id in EXTRA_DECK.iter() {
        unique_ids.insert(*id);
    }

    debug!("Fetching card data for {} unique cards", unique_ids.len());

    // 1. Fetch and register card data (must happen before createDuel)
    // Track which cards were successfully registered
    let mut loaded_ids: BTreeSet<u32> = BTreeSet::new();
    for id in unique_ids.iter() {
        if fetch_and_register_card(*id).await.is_ok() {
            loaded_ids.insert(*id);
        } else {
            debug!("Skipping card 0x{:08x} - failed to fetch data", id);
        }
    }

    debug!("Successfully loaded {} out of {} cards", loaded_ids.len(), unique_ids.len());

    // 2. Create the duel
    let duel = crate::ocgcore::create_duel()?;

    // 2.5 Preload support scripts and discovered dependencies before loading card scripts.
    preload_support_scripts(duel).await;

    debug!("Fetching and loading card scripts");

    // 3. Fetch and load scripts (after createDuel, before startDuel)
    for id in loaded_ids.iter() {
        let _ = fetch_and_load_script(duel, *id).await;
    }

    debug!("Populating decks");

    const POS_FACEDOWN_DEFENSE: u32 = 0x8;
    const LOCATION_DECK: u32 = 0x01;
    const LOCATION_EXTRA: u32 = 0x40;

    // Add only successfully loaded cards to the duel
    // Add main deck cards in REVERSE order (excluding unloaded cards)
    for i in (0..MAIN_DECK.len()).rev() {
        if loaded_ids.contains(&MAIN_DECK[i]) {
            crate::ocgcore::duel_new_card(duel, 0, 0, MAIN_DECK[i], 0, LOCATION_DECK, 0, POS_FACEDOWN_DEFENSE);
        }
    }

    // Add extra deck cards in REVERSE order (excluding unloaded cards)
    for i in (0..EXTRA_DECK.len()).rev() {
        if loaded_ids.contains(&EXTRA_DECK[i]) {
            crate::ocgcore::duel_new_card(duel, 0, 0, EXTRA_DECK[i], 0, LOCATION_EXTRA, 0, POS_FACEDOWN_DEFENSE);
        }
    }

    // 5. Start the duel
    debug!("Duel initialized successfully with loaded cards");
    crate::ocgcore::start_duel(duel);

    Ok(duel)
}
