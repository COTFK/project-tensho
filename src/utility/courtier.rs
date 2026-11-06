use super::card_database::CardDatabase;
use super::labels::build_card_label;
use super::{COURTIER_API_URL, courtier_card_id_batches, courtier_card_id_query};
use crate::ocgcore::CardType;
use ocgcore_ffi::constants::{
    ATTRIBUTE_DARK, ATTRIBUTE_DIVINE, ATTRIBUTE_EARTH, ATTRIBUTE_FIRE, ATTRIBUTE_LIGHT,
    ATTRIBUTE_WATER, ATTRIBUTE_WIND, RACE_AQUA, RACE_BEAST, RACE_BEASTWARRIOR, RACE_CREATORGOD,
    RACE_CYBERSE, RACE_DINOSAUR, RACE_DIVINE, RACE_DRAGON, RACE_FAIRY, RACE_FIEND, RACE_FISH,
    RACE_ILLUSION, RACE_INSECT, RACE_MACHINE, RACE_PLANT, RACE_PSYCHIC, RACE_PYRO, RACE_REPTILE,
    RACE_ROCK, RACE_SEASERPENT, RACE_SPELLCASTER, RACE_THUNDER, RACE_WARRIOR, RACE_WINGEDBEAST,
    RACE_WYRM, RACE_ZOMBIE,
};
use ocgcore_ffi::types::OCG_CardData;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct CourtierCardData {
    id: String,
    name: String,
    archetype: Option<Vec<String>>,
    r#type: String,
    race: Option<String>,
    subtypes: Option<Vec<String>>,
    level: Option<u32>,
    rank: Option<u32>,
    attribute: Option<String>,
    atk: Option<String>,
    def: Option<String>,
    scale: Option<u32>,
    linkval: Option<u32>,
    linkmarkers: Option<Vec<u32>>,
    labels: Option<CourtierLabels>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct CourtierLabels {
    name: Option<CourtierLabelName>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct CourtierLabelName {
    en: Option<String>,
}

impl CourtierCardData {
    fn display_name(&self) -> String {
        self.labels
            .as_ref()
            .and_then(|labels| labels.name.as_ref())
            .and_then(|name| name.en.clone())
            .unwrap_or_else(|| {
                tracing::debug!(card = %self.id, "no English label; using card name");
                self.name.clone()
            })
    }
}

const UNKNOWN_STAT: i32 = -2;

fn card_type(card: &CourtierCardData) -> anyhow::Result<u32> {
    let mut card_type = match card.r#type.as_str() {
        "Monster" => CardType::MONSTER,
        "Spell" => CardType::SPELL,
        "Trap" => CardType::TRAP,
        other => anyhow::bail!("unknown card type '{other}' on card {}", card.id),
    };
    for subtype in card.subtypes.iter().flatten() {
        card_type |= subtype_flag(subtype);
    }
    Ok(card_type.bits())
}

fn subtype_flag(subtype: &str) -> CardType {
    match subtype {
        "Normal" => CardType::NORMAL,
        "Effect" => CardType::EFFECT,
        "Fusion" => CardType::FUSION,
        "Ritual" => CardType::RITUAL,
        "Spirit" => CardType::SPIRIT,
        "Union" => CardType::UNION,
        "Gemini" => CardType::GEMINI,
        "Tuner" => CardType::TUNER,
        "Synchro" => CardType::SYNCHRO,
        "Flip" => CardType::FLIP,
        "Toon" => CardType::TOON,
        "Xyz" => CardType::XYZ,
        "Pendulum" => CardType::PENDULUM,
        "Link" => CardType::LINK,
        "Quick-Play" => CardType::QUICKPLAY,
        "Continuous" => CardType::CONTINUOUS,
        "Equip" => CardType::EQUIP,
        "Field" => CardType::FIELD,
        "Counter" => CardType::COUNTER,
        other => {
            tracing::warn!(subtype = %other, "unknown card subtype");
            CardType::empty()
        }
    }
}

fn level(card: &CourtierCardData) -> u32 {
    card.linkval.or(card.rank).or(card.level).unwrap_or(0)
}

fn attribute(value: Option<&str>) -> anyhow::Result<u32> {
    Ok(match value {
        None => 0,
        Some("EARTH") => ATTRIBUTE_EARTH,
        Some("WATER") => ATTRIBUTE_WATER,
        Some("FIRE") => ATTRIBUTE_FIRE,
        Some("WIND") => ATTRIBUTE_WIND,
        Some("LIGHT") => ATTRIBUTE_LIGHT,
        Some("DARK") => ATTRIBUTE_DARK,
        Some("DIVINE") => ATTRIBUTE_DIVINE,
        Some(other) => anyhow::bail!("unknown attribute '{other}'"),
    })
}

fn race(value: Option<&str>) -> anyhow::Result<u64> {
    Ok(match value {
        None => 0,
        Some("Warrior") => RACE_WARRIOR.into(),
        Some("Spellcaster") => RACE_SPELLCASTER.into(),
        Some("Fairy") => RACE_FAIRY.into(),
        Some("Fiend") => RACE_FIEND.into(),
        Some("Zombie") => RACE_ZOMBIE.into(),
        Some("Machine") => RACE_MACHINE.into(),
        Some("Aqua") => RACE_AQUA.into(),
        Some("Pyro") => RACE_PYRO.into(),
        Some("Rock") => RACE_ROCK.into(),
        Some("Winged Beast") => RACE_WINGEDBEAST.into(),
        Some("Plant") => RACE_PLANT.into(),
        Some("Insect") => RACE_INSECT.into(),
        Some("Thunder") => RACE_THUNDER.into(),
        Some("Dragon") => RACE_DRAGON.into(),
        Some("Beast") => RACE_BEAST.into(),
        Some("Beast-Warrior") => RACE_BEASTWARRIOR.into(),
        Some("Dinosaur") => RACE_DINOSAUR.into(),
        Some("Fish") => RACE_FISH.into(),
        Some("Sea Serpent") => RACE_SEASERPENT.into(),
        Some("Reptile") => RACE_REPTILE.into(),
        Some("Psychic") => RACE_PSYCHIC.into(),
        Some("Divine-Beast") => RACE_DIVINE.into(),
        Some("Creator God") => RACE_CREATORGOD.into(),
        Some("Wyrm") => RACE_WYRM.into(),
        Some("Cyberse") => RACE_CYBERSE.into(),
        Some("Illusion") => RACE_ILLUSION.into(),
        Some(other) => anyhow::bail!("unknown race '{other}'"),
    })
}

fn stat(value: Option<&str>) -> anyhow::Result<i32> {
    match value {
        None => Ok(0),
        Some("?") => Ok(UNKNOWN_STAT),
        Some(value) => Ok(value.parse()?),
    }
}

fn link_marker(markers: Option<&[u32]>) -> u32 {
    markers
        .unwrap_or_default()
        .iter()
        .fold(0, |mask, &marker| mask | (1 << (marker - 1)))
}

pub const ARCHETYPE_SCRIPT: &str = "archetype_setcode_constants.lua";

fn normalize_archetype(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn parse_archetype_setcodes(script: &str) -> anyhow::Result<HashMap<String, u16>> {
    let mut setcodes = HashMap::new();
    for line in script.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let Some(name) = name.trim().strip_prefix("SET_") else {
            continue;
        };
        let Some(hex) = value.trim().strip_prefix("0x") else {
            continue;
        };
        let setcode = u16::from_str_radix(hex, 16)?;
        if setcodes
            .insert(normalize_archetype(name), setcode)
            .is_some()
        {
            anyhow::bail!("duplicate archetype setcode '{name}'");
        }
    }
    Ok(setcodes)
}

// Each archetype is kept as its own setcode: the engine matches a query against
// every entry, so OR-ing codes together would match sets the card does not belong to.
fn card_setcodes(card: &CourtierCardData, archetypes: &HashMap<String, u16>) -> Vec<u16> {
    let mut setcodes = Vec::new();
    for series in card.archetype.iter().flatten() {
        if let Some(&setcode) = archetypes.get(&normalize_archetype(series)) {
            if !setcodes.contains(&setcode) {
                setcodes.push(setcode);
            }
        } else {
            tracing::warn!(card = %card.id, archetype = %series, "no setcode for archetype");
        }
    }
    setcodes
}

impl TryFrom<CourtierCardData> for OCG_CardData {
    type Error = anyhow::Error;

    fn try_from(card: CourtierCardData) -> anyhow::Result<Self> {
        Ok(OCG_CardData {
            code: card.id.parse()?,
            alias: 0,
            setcodes: std::ptr::null_mut(),
            r#type: card_type(&card)?,
            level: level(&card),
            attribute: attribute(card.attribute.as_deref())?,
            race: race(card.race.as_deref())?,
            attack: stat(card.atk.as_deref())?,
            defense: stat(card.def.as_deref())?,
            lscale: card.scale.unwrap_or(0),
            rscale: card.scale.unwrap_or(0),
            link_marker: link_marker(card.linkmarkers.as_deref()),
        })
    }
}

pub async fn fetch_card_data(
    card_ids: Vec<u32>,
    archetype_script: &[u8],
) -> anyhow::Result<CardDatabase> {
    let url = format!("{COURTIER_API_URL}/v1/cards");

    let client = reqwest::Client::new();
    let mut seen = HashSet::new();
    let requested: Vec<u32> = card_ids.into_iter().filter(|id| seen.insert(*id)).collect();

    let archetypes = parse_archetype_setcodes(std::str::from_utf8(archetype_script)?)?;

    let mut db = CardDatabase::new();

    for batch in courtier_card_id_batches(&requested) {
        let card_data = client
            .get(&url)
            .query(&courtier_card_id_query(batch))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<CourtierCardData>>()
            .await?;

        for card in card_data {
            let setcodes = card_setcodes(&card, &archetypes);
            let name = card.display_name();
            let data = OCG_CardData::try_from(card)?;
            let labels = build_card_label(data.code, name);
            db.insert(data.code, data, &setcodes, labels);
        }
    }

    let missing: Vec<u32> = requested
        .into_iter()
        .filter(|id| db.get(*id).is_none())
        .collect();
    if !missing.is_empty() {
        anyhow::bail!("Courtier returned no card data for: {missing:?}");
    }

    Ok(db)
}

#[cfg(test)]
#[path = "../../tests/unit/utility/courtier.rs"]
mod tests;
