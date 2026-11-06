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
