use super::*;
use crate::state::UIState;
use crate::ui::card_layer::{CardAddress, CardLayer};
use dioxus::core::{AttributeValue, ElementId, Mutation};
use std::cell::RefCell;

fn fixture(
    location: CardLocation,
    position: Option<CardPositionFlags>,
) -> (TrackedCard, CardTarget) {
    let address = CardAddress {
        controller: Player::Player,
        location,
        sequence: 0,
    };
    let mut layer = CardLayer::default();
    let id = layer.insert(address, 12345).unwrap();
    let mut card = *layer.card_at(address).unwrap();
    card.position = position;
    let target = CardTarget {
        id,
        anchor: CardAnchor {
            center_x: 120.0,
            center_y: 100.0,
            width: 59.0,
            height: 86.0,
        },
        data: CardData {
            card_code: 12345,
            controller: Player::Player,
            location,
            sequence: 0,
            ..Default::default()
        },
        rotation: 0.0,
        stack_order: 2,
        interactive: true,
    };
    (card, target)
}

#[test]
fn hidden_piles_overlays_and_unknown_faces_never_expose_card_art() {
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let (card, target) = fixture(location, Some(CardPositionFlags::FACE_UP_ATTACK));
        assert!(visible_card(card, Some(target)).is_none());
    }
    let (mut card, target) = fixture(CardLocation::MonsterZone, None);
    assert!(visible_card(card, Some(target)).unwrap().1);
    card.position = Some(CardPositionFlags::FACE_DOWN_DEFENSE);
    assert!(visible_card(card, Some(target)).unwrap().1);
    card.position = Some(CardPositionFlags::FACE_UP_DEFENSE);
    assert!(!visible_card(card, Some(target)).unwrap().1);
    card.code = 0;
    assert!(visible_card(card, Some(target)).unwrap().1);
    card.placement = CardPlacement::Overlay {
        host: card.id,
        sequence: 0,
    };
    assert!(visible_card(card, Some(target)).is_none());
}

#[test]
fn mismatched_target_is_not_rendered() {
    let (card, mut target) = fixture(CardLocation::Hand, None);
    assert!(!visible_card(card, Some(target)).unwrap().1);
    target.data.sequence = 1;
    assert!(visible_card(card, Some(target)).is_none());
    target.data.sequence = 0;
    target.data.card_code += 1;
    assert!(visible_card(card, Some(target)).is_none());
    target.data.card_code -= 1;
    target.anchor.width = f64::NAN;
    assert!(visible_card(card, Some(target)).is_none());
}

thread_local! {
    static TEST_STATE: RefCell<Option<Signal<(Vec<TrackedCard>, Vec<CardTarget>)>>> = const { RefCell::new(None) };
}

fn app() -> Element {
    use_context_provider(UIState::default);
    let (card, target) = fixture(CardLocation::Hand, None);
    let state = use_signal(|| (vec![card], vec![target]));
    TEST_STATE.with(|slot| *slot.borrow_mut() = Some(state));
    let (cards, targets) = state();
    rsx! { PersistentCardLayer { cards, targets, on_card_click: |_| {} } }
}

fn card_root(edits: &[Mutation], id: CardId) -> ElementId {
    edits
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetAttribute {
                name: "data-card-id",
                value: AttributeValue::Text(value),
                id: element,
                ..
            } if value == &id.to_string() => Some(*element),
            _ => None,
        })
        .expect("card root must have an identity attribute")
}

#[test]
fn movement_and_property_updates_keep_the_same_root_node() {
    let (mut card, mut target) = fixture(CardLocation::Hand, None);
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let root = card_root(&initial.edits, card.id);
    target.anchor.center_x += 80.0;
    target.rotation = -90.0;
    target.stack_order = 8;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let changes = dom.render_immediate_to_vec();
    assert!(
        !changes.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root
        )),
        "{:?}",
        changes.edits
    );
    assert!(changes.edits.iter().any(|edit| matches!(edit,
        Mutation::SetAttribute { name: "style", id, value: AttributeValue::Text(value), .. }
            if *id == root && value.contains("--card-z: 8;") && value.contains("--card-x: 170.5px;")
    )), "{:?}", changes.edits);

    card.code = 54321;
    target.data.card_code = 54321;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let updated = dom.render_immediate_to_vec();
    assert!(
        !updated.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root
        )),
        "{:?}",
        updated.edits
    );
    assert!(
        updated.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "src", value: AttributeValue::Text(value), .. }
                if value.contains("00054321")
        )),
        "{:?}",
        updated.edits
    );

    card.placement = CardPlacement::Zone(address(CardLocation::MonsterZone));
    card.position = Some(CardPositionFlags::FACE_DOWN_DEFENSE);
    target.data.location = CardLocation::MonsterZone;
    target.interactive = false;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let moved = dom.render_immediate_to_vec();
    assert!(
        !moved.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root
        )),
        "{:?}",
        moved.edits
    );
    assert!(
        moved.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "class", value: AttributeValue::Text(value), .. }
                if value.contains("pointer-events-none")
        )),
        "{:?}",
        moved.edits
    );
    assert!(
        moved.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "src", value: AttributeValue::Text(value), .. }
                if value.contains("cover") && !value.contains("00054321")
        )),
        "{:?}",
        moved.edits
    );
    let image_id = moved
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "src",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value.contains("cover") => Some(*id),
            _ => None,
        })
        .unwrap();
    card.placement = CardPlacement::Zone(CardAddress {
        sequence: 1,
        ..address(CardLocation::MonsterZone)
    });
    target.data.sequence = 1;
    target.anchor.center_x += 100.0;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let moved_again = dom.render_immediate_to_vec();
    assert!(!moved_again.edits.iter().any(|edit| matches!(edit,
        Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root || *id == image_id
    )), "{:?}", moved_again.edits);

    card.placement = CardPlacement::Zone(address(CardLocation::Deck));
    target.data.location = CardLocation::Deck;
    target.data.sequence = 0;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let hidden = dom.render_immediate_to_vec();
    assert!(
        !hidden.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root
        )),
        "{:?}",
        hidden.edits
    );
    assert!(
        hidden.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "class", value: AttributeValue::Text(value), id, .. }
                if *id == root && value.contains("hidden")
        )),
        "{:?}",
        hidden.edits
    );
    TEST_STATE.with(|slot| *slot.borrow_mut() = None);
}

fn address(location: CardLocation) -> CardAddress {
    CardAddress {
        controller: Player::Player,
        location,
        sequence: 0,
    }
}

#[test]
fn all_cards_keep_keyed_roots_when_reordered_and_piles_are_hidden() {
    let (card, target) = fixture(CardLocation::Hand, None);
    let mut layer = CardLayer::default();
    layer.insert(address(CardLocation::Hand), 12345).unwrap();
    let deck_address = address(CardLocation::Deck);
    layer.insert(deck_address, 98765).unwrap();
    let deck = *layer.card_at(deck_address).unwrap();

    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let first_root = card_root(&initial.edits, card.id);
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card, deck], vec![target])));
    let added = dom.render_immediate_to_vec();
    let deck_root = card_root(&added.edits, deck.id);
    assert!(!format!("{:?}", added.edits).contains("98765"));
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![deck, card], vec![target])));
    let reordered = dom.render_immediate_to_vec();
    assert!(!reordered.edits.iter().any(|edit| matches!(edit,
        Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == first_root || *id == deck_root
    )), "{:?}", reordered.edits);
    TEST_STATE.with(|slot| *slot.borrow_mut() = None);
}
