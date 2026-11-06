use super::*;

const ARCHETYPE_SCRIPT_TEXT: &str = include_str!("../../fixtures/archetype_setcode_constants.lua");

fn card_with(archetype: &[&str]) -> CourtierCardData {
    CourtierCardData {
        id: "00000001".into(),
        name: String::new(),
        archetype: Some(archetype.iter().map(|s| s.to_string()).collect()),
        r#type: "Monster".into(),
        race: None,
        subtypes: None,
        level: None,
        rank: None,
        attribute: None,
        atk: None,
        def: None,
        scale: None,
        linkval: None,
        linkmarkers: None,
        labels: Some(CourtierLabels {
            name: Some(CourtierLabelName {
                en: Some(String::new()),
            }),
        }),
    }
}

#[test]
fn labels_name_is_read_from_yaml_yugi_record() {
    let json = r#"{
            "id": "57554544",
            "name": "Fire King Island",
            "type": "Spell",
            "labels": { "name": { "en": "Fire King Island", "ja": "炎王の島" } }
        }"#;

    let card: CourtierCardData = serde_json::from_str(json).unwrap();

    assert_eq!(card.display_name(), "Fire King Island");
}

#[test]
fn card_labels_can_be_missing_or_null() {
    for labels in [
        "",
        r#", "labels": null"#,
        r#", "labels": {"name": null}"#,
        r#", "labels": {"name": {"en": null}}"#,
    ] {
        let json = format!(r#"{{"id":"1","name":"Test","type":"Monster"{labels}}}"#);
        let card: CourtierCardData = serde_json::from_str(&json).unwrap();
        assert_eq!(card.display_name(), "Test");
    }
}

fn spell_with_subtypes(subtypes: Option<Vec<&str>>) -> CourtierCardData {
    CourtierCardData {
        r#type: "Spell".into(),
        subtypes: subtypes.map(|s| s.iter().map(|s| s.to_string()).collect()),
        ..card_with(&[])
    }
}

#[test]
fn continuous_spell_carries_continuous_flag() {
    let card = spell_with_subtypes(Some(vec!["Continuous"]));

    assert_eq!(
        card_type(&card).unwrap(),
        (CardType::SPELL | CardType::CONTINUOUS).bits()
    );
}

#[test]
fn normal_spell_has_only_spell_flag() {
    let card = spell_with_subtypes(None);

    assert_eq!(card_type(&card).unwrap(), CardType::SPELL.bits());
}

#[test]
fn spell_and_trap_subtypes_map_to_their_flags() {
    assert_eq!(subtype_flag("Quick-Play"), CardType::QUICKPLAY);
    assert_eq!(subtype_flag("Equip"), CardType::EQUIP);
    assert_eq!(subtype_flag("Field"), CardType::FIELD);
    assert_eq!(subtype_flag("Counter"), CardType::COUNTER);
}

// Mirrors ocgcore's card::match_setcode: the base set must match and every bit of the query must be on the card.
fn engine_matches(card_setcode: u16, query: u16) -> bool {
    (card_setcode & 0xfff) == (query & 0xfff) && (query & card_setcode) == query
}

#[test]
fn parses_archetype_script() {
    let archetypes = parse_archetype_setcodes(ARCHETYPE_SCRIPT_TEXT).unwrap();
    assert_eq!(archetypes["ELEMENTALHERO"], 0x3008);
    assert_eq!(archetypes["RGENEX"], 0x1002);
    assert_eq!(archetypes["DD"], 0xaf);
}

#[test]
fn archetype_names_match_by_normalized_form() {
    let archetypes = parse_archetype_setcodes(ARCHETYPE_SCRIPT_TEXT).unwrap();
    let card = card_with(&["Elemental HERO", "R-Genex", "D/D", "Odd-Eyes"]);
    assert_eq!(
        card_setcodes(&card, &archetypes),
        vec![
            archetypes["ELEMENTALHERO"],
            archetypes["RGENEX"],
            archetypes["DD"],
            archetypes["ODDEYES"],
        ]
    );
}

#[test]
fn multi_archetype_card_keeps_one_setcode_per_archetype() {
    let archetypes = parse_archetype_setcodes(ARCHETYPE_SCRIPT_TEXT).unwrap();
    let setcodes = card_setcodes(&card_with(&["Elemental HERO", "Neos"]), &archetypes);
    assert_eq!(setcodes, vec![0x3008, 0x9]);
}

#[test]
fn unknown_archetype_is_skipped() {
    let archetypes = parse_archetype_setcodes(ARCHETYPE_SCRIPT_TEXT).unwrap();
    let setcodes = card_setcodes(&card_with(&["Not A Real Archetype", "Neos"]), &archetypes);
    assert_eq!(setcodes, vec![0x9]);
}

#[test]
fn engine_matching_respects_sub_archetypes() {
    let archetypes = parse_archetype_setcodes(ARCHETYPE_SCRIPT_TEXT).unwrap();
    let elemental = card_setcodes(&card_with(&["Elemental HERO"]), &archetypes);
    let vision_and_elemental =
        card_setcodes(&card_with(&["Elemental HERO", "Vision HERO"]), &archetypes);

    assert!(engine_matches(elemental[0], 0x8));
    assert!(engine_matches(elemental[0], 0x3008));
    assert!(!engine_matches(elemental[0], 0x5008));
    assert!(!engine_matches(0x8, 0x3008));
    assert!(engine_matches(vision_and_elemental[1], 0x5008));
    assert!(!engine_matches(vision_and_elemental[1], 0x3008));
}
