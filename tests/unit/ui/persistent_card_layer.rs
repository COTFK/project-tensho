use super::*;
use crate::duel::cards::{CardAddress, CardTracker};
use crate::state::UIState;
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
    let mut layer = CardTracker::default();
    let id = layer.insert(address, 12345).unwrap();
    let mut card = *layer.card_at(address).unwrap();
    card.position = position;
    let target = CardTarget {
        id,
        address,
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
        appearance: CardAppearance::Interactive,
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
fn invalid_geometry_is_not_rendered() {
    let (card, mut target) = fixture(CardLocation::Hand, None);
    assert!(!visible_card(card, Some(target)).unwrap().1);
    target.anchor.width = f64::NAN;
    assert!(visible_card(card, Some(target)).is_none());
}

#[test]
fn startup_artwork_matches_visible_card_images() {
    let (card, target) = fixture(CardLocation::Hand, None);
    assert_eq!(
        card_image(card, Some(target)),
        Some(CardImage::Face(target.data))
    );

    let (card, mut target) = fixture(
        CardLocation::MonsterZone,
        Some(CardPositionFlags::FACE_DOWN_DEFENSE),
    );
    assert_eq!(card_image(card, Some(target)), Some(CardImage::CardBack));
    target.data.position = Some(BattlePosition::FaceDownAttack);
    let mut face_up = card;
    face_up.position = Some(CardPositionFlags::FACE_UP_ATTACK);
    assert_eq!(card_image(face_up, Some(target)), Some(CardImage::CardBack));

    let (card, mut target) = fixture(CardLocation::ExtraDeck, None);
    target.appearance = CardAppearance::Pile(CardImage::ExtraDeckBack);
    assert_eq!(
        card_image(card, Some(target)),
        Some(CardImage::ExtraDeckBack)
    );
}

#[test]
fn chain_indicators_follow_cards_and_collect_buried_pile_links() {
    let mut layer = CardTracker::default();
    let field = layer
        .insert(address(CardLocation::MonsterZone), 10)
        .unwrap();
    let bottom = layer.insert(address(CardLocation::Graveyard), 11).unwrap();
    let top = layer
        .insert(
            CardAddress {
                sequence: 1,
                ..address(CardLocation::Graveyard)
            },
            12,
        )
        .unwrap();
    let links = [
        ChainLink {
            number: 1,
            card: Some(bottom),
        },
        ChainLink {
            number: 2,
            card: Some(field),
        },
        ChainLink {
            number: 3,
            card: Some(bottom),
        },
        ChainLink {
            number: 4,
            card: Some(top),
        },
        ChainLink {
            number: 5,
            card: None,
        },
    ];
    let indicators = chain_indicators(layer.cards(), &links);
    assert_eq!(indicators.get(&field), Some(&vec![2]));
    assert_eq!(indicators.get(&top), Some(&vec![1, 3, 4]));
    assert!(!indicators.contains_key(&bottom));

    layer
        .cards_mut()
        .iter_mut()
        .find(|card| card.id == bottom)
        .unwrap()
        .placement = CardPlacement::Zone(CardAddress {
        sequence: 0,
        ..address(CardLocation::Hand)
    });
    let moved = chain_indicators(layer.cards(), &links);
    assert_eq!(moved.get(&bottom), Some(&vec![1, 3]));
    assert_eq!(moved.get(&top), Some(&vec![4]));
}

#[test]
fn chain_indicators_for_overlay_materials_use_the_host() {
    let mut layer = CardTracker::default();
    let host = layer
        .insert(address(CardLocation::MonsterZone), 10)
        .unwrap();
    let material = layer.insert(address(CardLocation::Graveyard), 11).unwrap();
    layer
        .cards_mut()
        .iter_mut()
        .find(|card| card.id == material)
        .unwrap()
        .placement = CardPlacement::Overlay { host, sequence: 0 };
    assert_eq!(
        chain_indicators(
            layer.cards(),
            &[ChainLink {
                number: 2,
                card: Some(material)
            }]
        )
        .get(&host),
        Some(&vec![2])
    );
}

thread_local! {
    static TEST_STATE: RefCell<Option<Signal<(Vec<TrackedCard>, Vec<CardTarget>)>>> = const { RefCell::new(None) };
    static TEST_SELECTION: RefCell<Option<Signal<Option<CardId>>>> = const { RefCell::new(None) };
    static TEST_PLAYBACK: RefCell<Option<Signal<CardAnimationState>>> = const { RefCell::new(None) };
    static TEST_CHAIN: RefCell<Option<Signal<ChainPlayback>>> = const { RefCell::new(None) };
}

fn chain_badge_app() -> Element {
    use_context_provider(UIState::default);
    use_context_provider(ViewState::default);
    let chain = use_context_provider(|| Signal::new(ChainPlayback::default()));
    TEST_CHAIN.with(|slot| *slot.borrow_mut() = Some(chain));
    let (card, target) = fixture(
        CardLocation::MonsterZone,
        Some(CardPositionFlags::FACE_UP_ATTACK),
    );
    rsx! { PersistentCardLayer { cards: vec![card], targets: vec![target] } }
}

#[test]
fn rendered_card_displays_chain_numbers_and_removes_them_when_resolved() {
    let (card, _) = fixture(
        CardLocation::MonsterZone,
        Some(CardPositionFlags::FACE_UP_ATTACK),
    );
    let mut dom = VirtualDom::new(chain_badge_app);
    dom.rebuild_in_place();
    TEST_CHAIN.with(|slot| {
        slot.borrow().unwrap().write().displayed = vec![
            ChainLink {
                number: 1,
                card: Some(card.id),
            },
            ChainLink {
                number: 2,
                card: Some(card.id),
            },
        ];
    });
    let displayed = dom.render_immediate_to_vec();
    for number in [1, 2] {
        assert!(displayed.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "aria-label", value: AttributeValue::Text(value), .. }
                if value == &format!("Chain link {number}; higher links resolve first")
        )), "{:?}", displayed.edits);
    }
    TEST_CHAIN.with(|slot| slot.borrow().unwrap().write().displayed.clear());
    let cleared = dom.render_immediate_to_vec();
    assert!(
        cleared
            .edits
            .iter()
            .any(|edit| matches!(edit, Mutation::Remove { .. } | Mutation::ReplaceWith { .. })),
        "{:?}",
        cleared.edits
    );
}

fn field_app() -> Element {
    let mut state = use_context_provider(UIState::default);
    let view = use_context_provider(ViewState::default);
    let playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
    let (card, target) = fixture(
        CardLocation::MonsterZone,
        Some(CardPositionFlags::FACE_UP_ATTACK),
    );
    let mut layer = CardTracker::default();
    layer.insert(target.address, card.code).unwrap();
    state.cards.set(Some(layer));
    TEST_SELECTION.with(|slot| *slot.borrow_mut() = Some(view.selected_card));
    TEST_PLAYBACK.with(|slot| *slot.borrow_mut() = Some(playback));
    rsx! { PersistentCardLayer { cards: vec![card], targets: vec![target] } }
}

fn two_field_cards_app() -> Element {
    let mut state = use_context_provider(UIState::default);
    use_context_provider(ViewState::default);
    let playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
    let (first, first_target) = fixture(
        CardLocation::MonsterZone,
        Some(CardPositionFlags::FACE_UP_ATTACK),
    );
    let mut layer = CardTracker::default();
    layer.insert(first_target.address, first.code).unwrap();
    let second_address = CardAddress {
        sequence: 1,
        ..first_target.address
    };
    let second_id = layer.insert(second_address, 12346).unwrap();
    let mut second = *layer.card_by_id(second_id).unwrap();
    second.position = Some(CardPositionFlags::FACE_UP_ATTACK);
    let mut second_target = first_target;
    second_target.id = second_id;
    second_target.address = second_address;
    second_target.data.card_code = second.code;
    second_target.data.sequence = 1;
    state.cards.set(Some(layer));
    TEST_PLAYBACK.with(|slot| *slot.borrow_mut() = Some(playback));
    rsx! {
        PersistentCardLayer {
            cards: vec![first, second],
            targets: vec![first_target, second_target],
        }
    }
}

fn stats_root(edits: &[Mutation], card: CardId) -> ElementId {
    edits
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetAttribute {
                name: "data-card-stats",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value == &card.to_string() => Some(*id),
            _ => None,
        })
        .expect("field card must have a stats overlay")
}

#[test]
fn animation_hides_only_moving_field_card_stats_and_restores_them() {
    let mut layer = CardTracker::default();
    let first = layer
        .insert(address(CardLocation::MonsterZone), 12345)
        .unwrap();
    let second = layer
        .insert(
            CardAddress {
                sequence: 1,
                ..address(CardLocation::MonsterZone)
            },
            12346,
        )
        .unwrap();
    let hand = layer.insert(address(CardLocation::Hand), 12347).unwrap();
    let mut dom = VirtualDom::new(two_field_cards_app);
    let initial = dom.rebuild_to_vec();
    let first_stats = stats_root(&initial.edits, first);
    let second_stats = stats_root(&initial.edits, second);

    TEST_PLAYBACK.with(|slot| {
        slot.borrow()
            .unwrap()
            .write()
            .begin_gather(vec![hand], layer.clone());
    });
    let hand_shuffle = dom.render_immediate_to_vec();
    assert!(
        !hand_shuffle.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. }
                if *id == first_stats || *id == second_stats
        )),
        "{:?}",
        hand_shuffle.edits
    );

    TEST_PLAYBACK.with(|slot| {
        slot.borrow().unwrap().write().begin(vec![first], layer);
    });
    let moving = dom.render_immediate_to_vec();
    assert!(
        moving.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == first_stats
        )),
        "{:?}",
        moving.edits
    );
    assert!(
        !moving.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == second_stats
        )),
        "{:?}",
        moving.edits
    );

    TEST_PLAYBACK.with(|slot| {
        let mut playback = slot.borrow().unwrap();
        let generation = playback.peek().generation;
        playback.write().finish(generation);
    });
    let finished = dom.render_immediate_to_vec();
    stats_root(&finished.edits, first);
    assert!(
        !finished.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == second_stats
        )),
        "{:?}",
        finished.edits
    );
}

#[test]
fn selected_field_card_is_above_other_cards_and_respects_movement_layering() {
    let (card, _) = fixture(
        CardLocation::MonsterZone,
        Some(CardPositionFlags::FACE_UP_ATTACK),
    );
    let mut dom = VirtualDom::new(field_app);
    let initial = dom.rebuild_to_vec();
    let root = card_root(&initial.edits, card.id);

    TEST_SELECTION.with(|slot| slot.borrow().unwrap().set(Some(card.id)));
    let selected = dom.render_immediate_to_vec();
    assert!(
        selected.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "style", id, value: AttributeValue::Text(value), .. }
                if *id == root && value.contains("--card-z: 4500;")
        )),
        "{:?}",
        selected.edits
    );

    TEST_PLAYBACK.with(|slot| {
        slot.borrow()
            .unwrap()
            .write()
            .begin(vec![card.id], CardTracker::default());
    });
    let moving = dom.render_immediate_to_vec();
    assert!(
        moving.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "style", id, value: AttributeValue::Text(value), .. }
                if *id == root && value.contains("--card-z: 2500;")
        )),
        "{:?}",
        moving.edits
    );
}

fn app() -> Element {
    use_context_provider(UIState::default);
    use_context_provider(ViewState::default);
    let (card, target) = fixture(CardLocation::Hand, None);
    let state = use_signal(|| (vec![card], vec![target]));
    TEST_STATE.with(|slot| *slot.borrow_mut() = Some(state));
    let (cards, targets) = state();
    rsx! { PersistentCardLayer { cards, targets } }
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
    let original_image = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "src",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value.contains("00012345") => Some(*id),
            _ => None,
        })
        .unwrap();
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
            if *id == root && value.contains("--card-z: 1008;") && value.contains("--card-x: calc(170.5px + 0vw);")
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
    target.address.location = CardLocation::MonsterZone;
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
    assert_eq!(
        image_id, original_image,
        "hand-to-field move must retain the image node"
    );
    card.placement = CardPlacement::Zone(CardAddress {
        sequence: 1,
        ..address(CardLocation::MonsterZone)
    });
    target.data.sequence = 1;
    target.address.sequence = 1;
    target.anchor.center_x += 100.0;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let moved_again = dom.render_immediate_to_vec();
    assert!(!moved_again.edits.iter().any(|edit| matches!(edit,
        Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root || *id == image_id
    )), "{:?}", moved_again.edits);

    card.placement = CardPlacement::Zone(address(CardLocation::Deck));
    target.data.location = CardLocation::Deck;
    target.address.location = CardLocation::Deck;
    target.address.sequence = 0;
    target.data.sequence = 0;
    target.appearance = CardAppearance::Pile(CardImage::CardBack);
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let hidden = dom.render_immediate_to_vec();
    assert!(
        hidden.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "class", value: AttributeValue::Text(value), .. }
                if value.contains("pointer-events-none")
        )),
        "{:?}",
        hidden.edits
    );
    assert!(
        !hidden.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == root
        )),
        "{:?}",
        hidden.edits
    );
    assert!(
        !hidden.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == image_id
        )),
        "{:?}",
        hidden.edits
    );
    assert!(
        !hidden.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "src", value: AttributeValue::Text(value), .. }
                if value.contains("00054321")
        )),
        "{:?}",
        hidden.edits
    );
    let top = CardData {
        card_code: 67890,
        ..Default::default()
    };
    target.appearance = CardAppearance::Pile(CardImage::Thumbnail(top));
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let thumbnail = dom.render_immediate_to_vec();
    assert!(
        thumbnail.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "src", value: AttributeValue::Text(value), id, .. }
                if *id == image_id && *value == crate::utility::card_thumbnail_url(top.card_code)
        )),
        "{:?}",
        thumbnail.edits
    );
    assert!(!thumbnail.edits.iter().any(|edit| matches!(edit,
        Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == image_id || *id == root
    )), "{:?}", thumbnail.edits);
    card.placement = CardPlacement::Overlay {
        host: card.id,
        sequence: 0,
    };
    target.address.location = CardLocation::MonsterZone;
    target.data.location = CardLocation::MonsterZone;
    target.appearance = CardAppearance::Material;
    TEST_STATE.with(|slot| slot.borrow().unwrap().set((vec![card], vec![target])));
    let material = dom.render_immediate_to_vec();
    assert!(
        material.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "src", value: AttributeValue::Text(value), id, .. }
                if *id == image_id && value.contains("00054321")
        )),
        "{:?}",
        material.edits
    );
    assert!(!material.edits.iter().any(|edit| matches!(edit,
            Mutation::Remove { id } | Mutation::ReplaceWith { id, .. } if *id == image_id || *id == root
        )), "{:?}", material.edits);
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
    let mut layer = CardTracker::default();
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

#[test]
fn moving_between_hand_and_pile_does_not_reinsert_existing_roots() {
    let (mut card, mut target) = fixture(CardLocation::Hand, None);
    let mut layer = CardTracker::default();
    layer.insert(address(CardLocation::Hand), 12345).unwrap();
    layer.insert(address(CardLocation::Deck), 98765).unwrap();
    let deck = *layer.card_at(address(CardLocation::Deck)).unwrap();
    let (_, mut deck_target) = fixture(CardLocation::Deck, None);
    deck_target.id = deck.id;
    deck_target.data.card_code = deck.code;
    deck_target.appearance = CardAppearance::Pile(CardImage::CardBack);
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    let root = card_root(&initial.edits, card.id);
    TEST_STATE.with(|slot| {
        slot.borrow()
            .unwrap()
            .set((vec![card, deck], vec![target, deck_target]))
    });
    dom.render_immediate_to_vec();
    card.placement = CardPlacement::Zone(CardAddress {
        sequence: 1,
        ..address(CardLocation::Deck)
    });
    target.address = CardAddress {
        sequence: 1,
        ..address(CardLocation::Deck)
    };
    target.data.location = CardLocation::Deck;
    target.data.sequence = 1;
    target.stack_order = 1;
    target.appearance = CardAppearance::Pile(CardImage::CardBack);
    TEST_STATE.with(|slot| {
        slot.borrow()
            .unwrap()
            .set((vec![card, deck], vec![target, deck_target]))
    });
    let moved = dom.render_immediate_to_vec();
    assert!(
        !moved.edits.iter().any(|edit| matches!(edit,
            Mutation::PushRoot { id } if *id == root
        )),
        "{:?}",
        moved.edits
    );
    TEST_STATE.with(|slot| *slot.borrow_mut() = None);
}
