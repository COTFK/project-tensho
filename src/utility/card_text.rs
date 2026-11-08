use std::collections::HashMap;

use super::card_catalog::get_card_text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardText {
    pub name: String,
    pub optional_strings: HashMap<usize, String>,
}

pub fn build_card_text(code: u32, name: String) -> CardText {
    CardText {
        name,
        optional_strings: optional_strings(code),
    }
}

fn optional_strings(code: u32) -> HashMap<usize, String> {
    let entries: &[(usize, &str)] = match code {
        // Ulcanix
        44455560 => &[(2, "Make this card's Level become that added monster's?")],
        65305978 => &[(
            0,
            "Place 1 \"Fire King Island\" from your Deck face-up in your Field Zone?",
        )],
        // Fire King Island
        57554544 => &[
            (
                0,
                "Destroy 1 monster in your hand/field, and search 1 \"Fire King\" monster",
            ),
            (1, "Special Summon 1 FIRE Winged Beast from your hand"),
        ],
        // Arvata
        2526224 => &[(2, "Destroy 1 card on the field?")],
        // Extravagance
        49238328 => &[(0, "Banish 3 cards"), (1, "Banish 6 cards")],
        _ => &[],
    };

    entries
        .iter()
        .map(|(index, text)| (*index, text.to_string()))
        .collect()
}

pub fn get_optional_string_label(card_code: u32, string_index: usize) -> String {
    get_card_text(card_code)
        .and_then(|card_text| card_text.optional_strings.get(&string_index).cloned())
        .unwrap_or_else(|| String::from("error"))
}

#[cfg(test)]
#[path = "../../tests/unit/utility/card_text.rs"]
mod tests;
