use super::*;

fn set_data(cards: &mut CardTracker, address: CardAddress, data: CardData) {
    cards.update_queried_data(address, data).unwrap();
}

fn refresh_data(snapshot: &mut PreviewSnapshot) {
    for card in snapshot.cards.cards().to_vec() {
        if let CardPlacement::Zone(address) = card.placement {
            let mut data = card.data.unwrap_or_default();
            data.card_code = card.code;
            data.controller = address.controller;
            data.location = address.location;
            data.sequence = u8::try_from(address.sequence).unwrap();
            set_data(&mut snapshot.cards, address, data);
        }
    }
}

fn fixture() -> (PreviewSnapshot, PreviewGeometry) {
    let mut snapshot = PreviewSnapshot {
        cards: CardTracker::default(),
        selected: None,
    };
    let mut geometry = PreviewGeometry {
        anchors: HashMap::with_capacity(17),
        hand_base: CardAnchor {
            center_x: 550.0,
            center_y: 473.0,
            width: 100.0,
            height: 146.0,
        },
    };
    for location in [
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        geometry.anchors.insert(
            Anchor::Pile(location),
            CardAnchor {
                center_x: 629.5,
                center_y: 443.0,
                width: 59.0,
                height: 86.0,
            },
        );
    }
    for (location, count) in [
        (CardLocation::MonsterZone, 7),
        (CardLocation::SpellTrapZone, 6),
    ] {
        for sequence in 0..count {
            let address = CardAddress {
                controller: Player::Player,
                location,
                sequence,
            };
            let code = 100 + sequence;
            snapshot.cards.insert(address, code).unwrap();
            let data = CardData {
                card_code: code,
                controller: Player::Player,
                location,
                sequence: u8::try_from(sequence).unwrap(),
                ..Default::default()
            };
            set_data(&mut snapshot.cards, address, data);
            geometry.anchors.insert(
                Anchor::Zone(location, sequence),
                CardAnchor {
                    center_x: f64::from(sequence) * 100.0 + 29.5,
                    center_y: 63.0,
                    width: 59.0,
                    height: 86.0,
                },
            );
        }
    }
    (snapshot, geometry)
}

fn all_locations_fixture() -> (PreviewSnapshot, PreviewGeometry) {
    let (mut snapshot, geometry) = fixture();
    for location in [
        CardLocation::Hand,
        CardLocation::Deck,
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        snapshot
            .cards
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location,
                    sequence: 0,
                },
                42,
            )
            .unwrap();
    }
    refresh_data(&mut snapshot);
    (snapshot, geometry)
}

#[test]
fn field_targets_preserve_identity_geometry_and_queried_data() {
    let (snapshot, geometry) = fixture();
    let expected: Vec<_> = snapshot
        .cards
        .cards()
        .iter()
        .map(|card| {
            let CardPlacement::Zone(address) = card.placement else {
                unreachable!()
            };
            CardTarget {
                id: card.id,
                address,
                anchor: geometry.anchors[&Anchor::Zone(address.location, address.sequence)],
                data: card.data.unwrap(),
                rotation: 0.0,
                stack_order: 2,
                appearance: CardAppearance::Interactive,
            }
        })
        .collect();
    assert_eq!(expected.len(), 13);
    assert_eq!(targets(&snapshot, &geometry).unwrap(), expected);
}

#[test]
fn geometry_identity_and_target_order_do_not_depend_on_map_insertion_order() {
    let (input, mut geometry) = all_locations_fixture();
    let keys: Vec<_> = input
        .cards
        .cards()
        .iter()
        .filter_map(|card| {
            let CardPlacement::Zone(address) = card.placement else {
                unreachable!()
            };
            (address.location != CardLocation::Hand).then(|| Anchor::try_from(address).unwrap())
        })
        .collect();
    assert_eq!(keys.len(), 17);
    let entries: Vec<_> = keys
        .into_iter()
        .enumerate()
        .map(|(index, anchor)| {
            let index = index as f64;
            (
                anchor,
                CardAnchor {
                    center_x: 100.0 * index,
                    center_y: 20.0 + index,
                    width: 59.0 + index,
                    height: 86.0 + index,
                },
            )
        })
        .collect();
    let mut expected_targets = None::<Vec<CardTarget>>;
    for order in 0..3 {
        let mut ordered = entries.clone();
        match order {
            0 => {}
            1 => ordered.reverse(),
            _ => ordered.rotate_left(5),
        }
        geometry.anchors = ordered.into_iter().collect();
        let actual = targets(&input, &geometry).unwrap();
        assert_eq!(actual.len(), 18);
        for card in input.cards.cards() {
            let CardPlacement::Zone(address) = card.placement else {
                unreachable!()
            };
            let expected_anchor = if address.location == CardLocation::Hand {
                geometry.hand_base
            } else {
                let key = Anchor::try_from(address).unwrap();
                entries.iter().find(|(anchor, _)| *anchor == key).unwrap().1
            };
            let target = actual.iter().find(|target| target.id == card.id).unwrap();
            assert_eq!(target.address, address);
            assert_eq!(target.anchor, expected_anchor);
            assert_eq!(
                target.rotation,
                if address.location == CardLocation::Banishment {
                    -90.0
                } else {
                    0.0
                }
            );
        }
        if let Some(expected) = &expected_targets {
            assert_eq!(&actual, expected);
        } else {
            expected_targets = Some(actual);
        }
    }
}

#[test]
fn missing_geometry_preserves_field_and_pile_errors() {
    let (input, mut geometry) = all_locations_fixture();
    let keys: Vec<_> = geometry.anchors.keys().copied().collect();
    assert_eq!(keys.len(), 17);
    for anchor in keys {
        let removed = geometry.anchors.remove(&anchor).unwrap();
        let error = targets(&input, &geometry).unwrap_err();
        assert_eq!(
            error.to_string(),
            match anchor {
                Anchor::Zone(_, _) => "Field anchor missing",
                Anchor::Pile(_) => "Pile anchor missing",
                Anchor::HandBase => unreachable!(),
            }
        );
        geometry.anchors.insert(anchor, removed);
    }
}

#[test]
fn invalid_or_missing_tracking_never_produces_a_snapshot() {
    let (mut input, _) = fixture();
    assert!(snapshot(None, input.selected).is_none());
    assert!(snapshot(Some(input.cards.clone()), input.selected).is_some());
    input
        .cards
        .insert(
            CardAddress {
                controller: Player::Player,
                location: CardLocation::Hand,
                sequence: 0,
            },
            42,
        )
        .unwrap();
    assert!(snapshot(Some(input.cards), input.selected).is_none());
}

#[test]
fn unsupported_occupied_field_slots_fail_instead_of_disappearing() {
    for (location, sequence) in [
        (CardLocation::MonsterZone, 7),
        (CardLocation::SpellTrapZone, 6),
    ] {
        let (mut input, mut geometry) = fixture();
        let address = CardAddress {
            controller: Player::Player,
            location,
            sequence,
        };
        input.cards.insert(address, 1).unwrap();
        refresh_data(&mut input);
        geometry
            .anchors
            .insert(Anchor::Zone(location, sequence), geometry.hand_base);
        let error = targets(&input, &geometry).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("No visible field zone for {address:?}")
        );
    }
}

#[test]
fn hand_and_piles_render_in_sequence_order() {
    let (_, geometry) = fixture();
    for location in [CardLocation::Hand, CardLocation::Graveyard] {
        let mut snapshot = PreviewSnapshot {
            cards: CardTracker::default(),
            selected: None,
        };
        let address = CardAddress {
            controller: Player::Player,
            location,
            sequence: 1,
        };
        let second = snapshot.cards.insert(address, 2).unwrap();
        refresh_data(&mut snapshot);
        let first = snapshot
            .cards
            .insert(
                CardAddress {
                    sequence: 0,
                    ..address
                },
                1,
            )
            .unwrap();
        refresh_data(&mut snapshot);
        let targets = targets(&snapshot, &geometry).unwrap();
        assert_eq!(
            targets.iter().map(|target| target.id).collect::<Vec<_>>(),
            [first, second]
        );
    }
}

#[test]
fn hand_fan_uses_slot_spacing_rotation_and_selected_lift() {
    let base = CardAnchor {
        center_x: 500.0,
        center_y: 600.0,
        width: 100.0,
        height: 146.0,
    };
    let (left, rotation, stack) = hand_position(base, 0, 3, false);
    assert_eq!(rotation, -2.0);
    assert_eq!(stack, 0);
    let shift = 146.0 / 100.0;
    assert!((left.center_x - (422.0 - (-2.0_f64).to_radians().sin() * shift)).abs() < 0.00001);
    assert!((left.center_y - (600.0 + (-2.0_f64).to_radians().cos() * shift)).abs() < 0.00001);
    let (selected, rotation, stack) = hand_position(base, 1, 3, true);
    assert_eq!(rotation, 0.0);
    assert_eq!(stack, 3);
    assert_eq!(selected.center_x, 500.0);
    assert_eq!(selected.center_y, 563.5);
    let resized = CardAnchor {
        width: 200.0,
        height: 292.0,
        ..base
    };
    let (resized_left, _, _) = hand_position(resized, 0, 3, false);
    assert!((resized_left.center_x - (500.0 + 2.0 * (left.center_x - 500.0))).abs() < 0.00001);
}

#[test]
fn duplicate_hand_cards_keep_identity_when_one_moves_and_the_rest_reindex() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{CoreMessage, MoveMessageData};

    let (_, geometry) = fixture();
    let mut snapshot = PreviewSnapshot {
        cards: CardTracker::default(),
        selected: None,
    };
    for sequence in 0..3 {
        let address = CardAddress {
            controller: Player::Player,
            location: CardLocation::Hand,
            sequence,
        };
        snapshot.cards.insert(address, 42).unwrap();
    }
    refresh_data(&mut snapshot);

    fn pile_targets_preserve_ids_counts_and_concealment() {
        let (_, geometry) = fixture();
        let mut snapshot = PreviewSnapshot {
            cards: CardTracker::default(),
            selected: None,
        };
        for sequence in 0..2 {
            snapshot
                .cards
                .insert(
                    CardAddress {
                        controller: Player::Player,
                        location: CardLocation::Deck,
                        sequence,
                    },
                    100 + sequence,
                )
                .unwrap();
        }
        refresh_data(&mut snapshot);
        let before = targets(&snapshot, &geometry).unwrap();
        assert_eq!(before.len(), 2);
        assert!(
            before
                .iter()
                .all(|target| target.appearance == CardAppearance::Pile(CardImage::CardBack))
        );

        use crate::ocgcore::location_info::{
            CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
        };
        use crate::ocgcore::messages::draw::DrawCard;
        use crate::ocgcore::messages::move_card::MoveReason;
        use crate::ocgcore::messages::{CoreMessage, DrawMessageData, MoveMessageData};
        snapshot
            .cards
            .apply(&CoreMessage::Draw(DrawMessageData {
                player: Player::Player,
                cards: vec![DrawCard {
                    code: 101,
                    position: CardPositionFlags::FACE_DOWN_ATTACK,
                }],
            }))
            .unwrap();
        refresh_data(&mut snapshot);
        let drawn = targets(&snapshot, &geometry).unwrap();
        assert_eq!(
            drawn
                .iter()
                .find(|target| target.data.location == CardLocation::Hand)
                .unwrap()
                .id,
            before[1].id
        );
        let loc = |location| CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Known(location),
            sequence: 0,
            position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
        };
        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 101,
                previous: loc(CardLocation::Hand),
                current: loc(CardLocation::Graveyard),
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
        refresh_data(&mut snapshot);
        let discarded = targets(&snapshot, &geometry).unwrap();
        let grave = discarded
            .iter()
            .find(|target| target.data.location == CardLocation::Graveyard)
            .unwrap();
        assert_eq!(grave.id, before[1].id);
        assert_eq!(
            grave.appearance,
            CardAppearance::Pile(CardImage::Thumbnail(
                snapshot.cards.zone_data(CardLocation::Graveyard).unwrap()[0].unwrap()
            ))
        );

        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 101,
                previous: loc(CardLocation::Graveyard),
                current: CardLocationInfo {
                    position: CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE),
                    ..loc(CardLocation::Banishment)
                },
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
        refresh_data(&mut snapshot);
        set_data(
            &mut snapshot.cards,
            CardAddress {
                controller: Player::Player,
                location: CardLocation::Banishment,
                sequence: 0,
            },
            CardData {
                card_code: 101,
                controller: Player::Player,
                location: CardLocation::Banishment,
                position: Some(BattlePosition::FaceDownDefense),
                ..Default::default()
            },
        );
        let banished = targets(&snapshot, &geometry).unwrap();
        let banished = banished
            .iter()
            .find(|target| target.data.location == CardLocation::Banishment)
            .unwrap();
        assert_eq!(banished.id, before[1].id);
        assert_eq!(
            banished.appearance,
            CardAppearance::Pile(CardImage::CardBack)
        );
    }

    fn material_targets_follow_host_and_keep_identity_when_detached() {
        use crate::ocgcore::location_info::{
            CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
        };
        use crate::ocgcore::messages::move_card::MoveReason;
        use crate::ocgcore::messages::{CoreMessage, MoveMessageData};
        let (_, geometry) = fixture();
        let mut snapshot = PreviewSnapshot {
            cards: CardTracker::default(),
            selected: None,
        };
        let address = |location| CardAddress {
            controller: Player::Player,
            location,
            sequence: 0,
        };
        snapshot
            .cards
            .insert(address(CardLocation::MonsterZone), 10)
            .unwrap();
        let material_id = snapshot
            .cards
            .insert(address(CardLocation::Hand), 20)
            .unwrap();
        refresh_data(&mut snapshot);
        let ordinary = |location| CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Known(location),
            sequence: 0,
            position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
        };
        let overlay = CardLocationInfo {
            controller: Some(Player::Player),
            location: CardLocationKind::Overlay {
                host: Some(CardLocation::MonsterZone),
            },
            sequence: 0,
            position: CardLocationPosition::OverlaySequence(0),
        };
        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 20,
                previous: ordinary(CardLocation::Hand),
                current: overlay,
                reason: MoveReason::XYZ | MoveReason::MATERIAL,
            }))
            .unwrap();
        let attached = targets(&snapshot, &geometry).unwrap();
        let material = attached
            .iter()
            .find(|target| target.id == material_id)
            .unwrap();
        assert_eq!(material.appearance, CardAppearance::Material);
        assert_eq!(material.data.card_code, 20);
        snapshot
            .cards
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: 20,
                previous: overlay,
                current: ordinary(CardLocation::Graveyard),
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
        refresh_data(&mut snapshot);
        let detached = targets(&snapshot, &geometry).unwrap();
        assert_eq!(
            detached
                .iter()
                .find(|target| target.data.location == CardLocation::Graveyard)
                .unwrap()
                .id,
            material_id
        );
    }
    pile_targets_preserve_ids_counts_and_concealment();
    material_targets_follow_host_and_keep_identity_when_detached();
    let before = targets(&snapshot, &geometry).unwrap();
    assert_eq!(before.len(), 3);
    assert_ne!(before[0].id, before[1].id);
    assert_ne!(before[1].id, before[2].id);
    let loc = |location, sequence| CardLocationInfo {
        controller: Some(Player::Player),
        location: CardLocationKind::Known(location),
        sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    };
    snapshot
        .cards
        .apply(&CoreMessage::Move(MoveMessageData {
            card_code: 42,
            previous: loc(CardLocation::Hand, 0),
            current: loc(CardLocation::MonsterZone, 0),
            reason: MoveReason::EFFECT,
        }))
        .unwrap();
    refresh_data(&mut snapshot);
    let after = targets(&snapshot, &geometry).unwrap();
    assert_eq!(after[0].id, before[0].id);
    assert_eq!(after[1].id, before[1].id);
    assert_eq!(after[2].id, before[2].id);
    assert_eq!(after[1].data.sequence, 0);
    assert_eq!(after[2].data.sequence, 1);
    assert_eq!(after[1].rotation, -1.0);
    assert_eq!(after[2].rotation, 1.0);
}

#[test]
fn incoming_graveyard_cards_do_not_replace_the_stationary_pile_image() {
    use crate::ocgcore::location_info::{
        CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
    };
    use crate::ocgcore::messages::move_card::MoveReason;
    use crate::ocgcore::messages::{CoreMessage, MoveMessageData};
    let (_, geometry) = fixture();
    let mut snapshot = PreviewSnapshot {
        cards: CardTracker::default(),
        selected: None,
    };
    let address = |location, sequence| CardAddress {
        controller: Player::Player,
        location,
        sequence,
    };
    let stationary = snapshot
        .cards
        .insert(address(CardLocation::Graveyard, 0), 10)
        .unwrap();
    let moving = snapshot
        .cards
        .insert(address(CardLocation::Hand, 0), 20)
        .unwrap();
    refresh_data(&mut snapshot);
    let mut playback = CardAnimationState::default();
    playback.begin(vec![moving], snapshot.cards.clone());
    let loc = |location, sequence| CardLocationInfo {
        controller: Some(Player::Player),
        location: CardLocationKind::Known(location),
        sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    };
    snapshot
        .cards
        .apply(&CoreMessage::Move(MoveMessageData {
            card_code: 20,
            previous: loc(CardLocation::Hand, 0),
            current: loc(CardLocation::Graveyard, 1),
            reason: MoveReason::EFFECT,
        }))
        .unwrap();
    refresh_data(&mut snapshot);
    let mut rendered = targets(&snapshot, &geometry).unwrap();
    preserve_pile_images(&snapshot, &mut rendered, &playback).unwrap();
    assert_eq!(
        rendered
            .iter()
            .find(|target| target.id == stationary)
            .unwrap()
            .appearance,
        CardAppearance::Pile(CardImage::Thumbnail(
            snapshot
                .cards
                .card_at(address(CardLocation::Graveyard, 0))
                .unwrap()
                .data
                .unwrap()
        ))
    );
    assert_eq!(
        rendered
            .iter()
            .find(|target| target.id == moving)
            .unwrap()
            .appearance,
        CardAppearance::Pile(CardImage::Thumbnail(CardData {
            card_code: 20,
            controller: Player::Player,
            location: CardLocation::Graveyard,
            sequence: 1,
            ..Default::default()
        }))
    );
    playback.finish(playback.generation);
    let mut completed = targets(&snapshot, &geometry).unwrap();
    preserve_pile_images(&snapshot, &mut completed, &playback).unwrap();
    assert_eq!(
        completed
            .iter()
            .find(|target| target.id == stationary)
            .unwrap()
            .appearance,
        CardAppearance::Pile(CardImage::Thumbnail(
            snapshot
                .cards
                .card_at(address(CardLocation::Graveyard, 1))
                .unwrap()
                .data
                .unwrap()
        ))
    );
}

#[test]
fn gathering_stacks_hand_targets_until_spread() {
    let (_, geometry) = fixture();
    let mut snapshot = PreviewSnapshot {
        cards: CardTracker::default(),
        selected: None,
    };
    for sequence in 0..3 {
        snapshot
            .cards
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Hand,
                    sequence,
                },
                10 + sequence,
            )
            .unwrap();
    }

    refresh_data(&mut snapshot);
    let spread_anchors: Vec<_> = targets(&snapshot, &geometry)
        .unwrap()
        .iter()
        .map(|target| target.anchor)
        .collect();
    let mut playback = CardAnimationState::default();
    playback.begin_gather(vec![], snapshot.cards.clone());
    let mut gathered = targets(&snapshot, &geometry).unwrap();
    gather_hand(geometry.hand_base, &mut gathered, &playback);
    assert!(
        gathered
            .iter()
            .all(|target| { target.anchor == geometry.hand_base && target.rotation == 0.0 })
    );
    playback.advance(playback.generation, AnimationPhase::HandShuffleGathering);
    playback.advance(playback.generation, AnimationPhase::HandShuffleStacking);
    let mut spread = targets(&snapshot, &geometry).unwrap();
    gather_hand(geometry.hand_base, &mut spread, &playback);
    assert_eq!(
        spread
            .iter()
            .map(|target| target.anchor)
            .collect::<Vec<_>>(),
        spread_anchors
    );
}

#[test]
fn shuffle_gathers_original_cards_then_spreads_shuffled_cards() {
    use crate::ocgcore::messages::{CoreMessage, ShuffleCardsMessageData};

    let (_, geometry) = fixture();
    let mut before = PreviewSnapshot {
        cards: CardTracker::default(),
        selected: None,
    };
    for (sequence, code) in [10, 20].into_iter().enumerate() {
        before
            .cards
            .insert(
                CardAddress {
                    controller: Player::Player,
                    location: CardLocation::Hand,
                    sequence: sequence as u32,
                },
                code,
            )
            .unwrap();
    }
    refresh_data(&mut before);
    let mut after = before.clone();
    after
        .cards
        .apply(&CoreMessage::ShuffleHand(ShuffleCardsMessageData {
            player: Player::Player,
            card_codes: vec![20, 10],
        }))
        .unwrap();
    after
        .cards
        .insert(
            CardAddress {
                controller: Player::Player,
                location: CardLocation::Hand,
                sequence: 2,
            },
            30,
        )
        .unwrap();
    refresh_data(&mut after);

    let mut playback = CardAnimationState::default();
    playback.begin_gather(vec![], before.cards.clone());
    let mut cards = after.cards.cards().to_vec();
    shuffle_source_cards(&mut cards, &playback);
    assert_eq!(cards, before.cards.cards());
    let mut gathered = targets(&after, &geometry).unwrap();
    shuffle_source_targets(&mut gathered, geometry.hand_base, &playback).unwrap();
    gather_hand(geometry.hand_base, &mut gathered, &playback);
    assert_eq!(gathered.len(), 2);
    for (target, card) in gathered.iter().zip(before.cards.cards()) {
        assert_eq!(target.id, card.id);
        assert_eq!(target.data.card_code, card.code);
        assert_eq!(target.anchor, geometry.hand_base);
    }

    playback.advance(playback.generation, AnimationPhase::HandShuffleGathering);
    let mut stacked = targets(&after, &geometry).unwrap();
    gather_hand(geometry.hand_base, &mut stacked, &playback);
    assert!(
        stacked
            .iter()
            .all(|target| target.anchor == geometry.hand_base)
    );
    assert_eq!(
        stacked
            .iter()
            .map(|target| target.data.card_code)
            .collect::<Vec<_>>(),
        vec![20, 10, 30]
    );
    playback.advance(playback.generation, AnimationPhase::HandShuffleStacking);
    let spread = targets(&after, &geometry).unwrap();
    assert_eq!(spread.len(), 3);
    assert_ne!(spread[0].anchor, spread[1].anchor);
    assert_ne!(spread[1].anchor, spread[2].anchor);
}
