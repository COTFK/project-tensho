use super::*;

#[test]
fn known_card_text_contains_optional_strings() {
    let label = build_card_text(57554544, "Fire King Island".into());

    assert_eq!(label.optional_strings.len(), 2);
}

#[test]
fn unknown_card_has_no_optional_strings() {
    assert!(
        build_card_text(1, "Test".into())
            .optional_strings
            .is_empty()
    );
}
