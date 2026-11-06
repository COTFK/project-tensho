use super::*;
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::messages::draw::DrawCard;
use crate::ocgcore::messages::move_card::MoveReason;
use crate::ocgcore::messages::{
    PositionChangeMessageData, SetCardMessageData, SummoningMessageData, SwapMessageData,
};

fn address(controller: Player, location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller,
        location,
        sequence,
    }
}

fn location(address: Option<CardAddress>) -> CardLocationInfo {
    CardLocationInfo {
        controller: Some(address.map_or(Player::Player, |address| address.controller)),
        location: address.map_or(CardLocationKind::None, |address| {
            CardLocationKind::Known(address.location)
        }),
        sequence: address.map_or(0, |address| address.sequence),
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    }
}

fn movement(from: Option<CardAddress>, to: Option<CardAddress>, code: u32) -> CoreMessage {
    CoreMessage::Move(MoveMessageData {
        card_code: code,
        previous: location(from),
        current: location(to),
        reason: MoveReason::EFFECT,
    })
}

fn overlay(host: Option<CardAddress>, sequence: u32) -> CardLocationInfo {
    CardLocationInfo {
        controller: host.map(|address| address.controller),
        location: CardLocationKind::Overlay {
            host: host.map(|address| address.location),
        },
        sequence: host.map_or(0, |address| address.sequence),
        position: CardLocationPosition::OverlaySequence(sequence),
    }
}

fn overlay_move(previous: CardLocationInfo, current: CardLocationInfo, code: u32) -> CoreMessage {
    CoreMessage::Move(MoveMessageData {
        card_code: code,
        previous,
        current,
        reason: MoveReason::XYZ | MoveReason::MATERIAL,
    })
}

fn draw(player: Player, codes: &[u32]) -> CoreMessage {
    CoreMessage::Draw(DrawMessageData {
        player,
        cards: codes
            .iter()
            .map(|&code| DrawCard {
                code,
                position: CardPositionFlags::FACE_DOWN_ATTACK
                    | CardPositionFlags::FACE_DOWN_DEFENSE,
            })
            .collect(),
    })
}

fn codes(layer: &CardLayer, player: Player, location: CardLocation) -> Vec<u32> {
    let len = layer.pile_len(player, location).unwrap();
    (0..len)
        .map(|sequence| {
            layer
                .card_at(address(player, location, sequence))
                .unwrap()
                .code
        })
        .collect()
}

fn position_change(
    address: CardAddress,
    code: u32,
    previous_position: CardPositionFlags,
    current_position: CardPositionFlags,
) -> CoreMessage {
    CoreMessage::PositionChange(PositionChangeMessageData {
        card_code: code,
        controller: address.controller,
        location: address.location,
        sequence: address.sequence,
        previous_position,
        current_position,
    })
}

#[test]
fn swaps_occupied_zones_and_carries_materials_with_their_hosts() {
    let first = address(Player::Player, CardLocation::MonsterZone, 0);
    let second = address(Player::Opponent, CardLocation::MonsterZone, 3);
    let first_material = address(Player::Player, CardLocation::Hand, 0);
    let second_material = address(Player::Opponent, CardLocation::Hand, 0);
    let mut layer = CardLayer::default();
    let first_id = layer.insert(first, 100).unwrap();
    let second_id = layer.insert(second, 0).unwrap();
    let first_material_id = layer.insert(first_material, 101).unwrap();
    let second_material_id = layer.insert(second_material, 201).unwrap();
    for (source, host, code) in [(first_material, first, 101), (second_material, second, 201)] {
        layer
            .apply(&overlay_move(
                location(Some(source)),
                overlay(Some(host), 0),
                code,
            ))
            .unwrap();
    }
    let mut first_location = location(Some(first));
    first_location.position = CardLocationPosition::Position(CardPositionFlags::FACE_UP_DEFENSE);
    let mut second_location = location(Some(second));
    second_location.position = CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE);
    layer
        .apply(&CoreMessage::Swap(SwapMessageData {
            first_code: 0,
            first_location,
            second_code: 200,
            second_location,
        }))
        .unwrap();
    let first_card = layer.card_at(second).unwrap();
    assert_eq!((first_card.id, first_card.code), (first_id, 100));
    assert_eq!(
        first_card.position,
        Some(CardPositionFlags::FACE_UP_DEFENSE)
    );
    let second_card = layer.card_at(first).unwrap();
    assert_eq!((second_card.id, second_card.code), (second_id, 200));
    assert_eq!(
        second_card.position,
        Some(CardPositionFlags::FACE_DOWN_DEFENSE)
    );
    assert_eq!(
        layer.material_at(first_id, 0).unwrap().id,
        first_material_id
    );
    assert_eq!(
        layer.material_at(second_id, 0).unwrap().id,
        second_material_id
    );

    let grave = address(Player::Opponent, CardLocation::Graveyard, 0);
    layer
        .apply(&overlay_move(
            overlay(Some(second), 0),
            location(Some(grave)),
            101,
        ))
        .unwrap();
    assert_eq!(layer.card_at(grave).unwrap().id, first_material_id);
    assert_eq!(
        layer.material_at(second_id, 0).unwrap().id,
        second_material_id
    );
}

#[test]
fn rejects_invalid_swaps_atomically() {
    let first = address(Player::Player, CardLocation::MonsterZone, 0);
    let second = address(Player::Player, CardLocation::MonsterZone, 1);
    let missing = address(Player::Player, CardLocation::MonsterZone, 2);
    let pile = address(Player::Player, CardLocation::Hand, 0);
    let mut layer = CardLayer::default();
    layer.insert(first, 0).unwrap();
    layer.insert(second, 200).unwrap();
    layer.insert(pile, 300).unwrap();
    let before = layer.clone();
    for (first_location, second_location, second_code) in [
        (location(Some(first)), location(Some(missing)), 0),
        (location(Some(first)), location(Some(second)), 999),
        (location(Some(first)), location(Some(first)), 0),
        (location(Some(first)), location(None), 0),
        (location(Some(first)), location(Some(pile)), 0),
        (location(Some(first)), overlay(Some(second), 0), 0),
    ] {
        assert!(
            layer
                .apply(&CoreMessage::Swap(SwapMessageData {
                    first_code: 100,
                    first_location,
                    second_code,
                    second_location,
                }))
                .is_err()
        );
        assert_eq!(layer, before);
    }
}

#[test]
fn tracks_positions_and_reveals_without_replacing_identity() {
    let field = address(Player::Player, CardLocation::MonsterZone, 2);
    let mut layer = CardLayer::default();
    let id = layer.insert(field, 0).unwrap();
    assert_eq!(layer.card_at(field).unwrap().position, None);

    let mut info = location(Some(field));
    info.position = CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE);
    layer
        .apply(&CoreMessage::SetCard(SetCardMessageData {
            card_code: 0,
            location: info,
        }))
        .unwrap();
    let set = layer.card_at(field).unwrap();
    assert_eq!(set.id, id);
    assert_eq!(set.code, 0);
    assert_eq!(set.position, Some(CardPositionFlags::FACE_DOWN_DEFENSE));

    layer
        .apply(&position_change(
            field,
            42,
            CardPositionFlags::FACE_DOWN_DEFENSE,
            CardPositionFlags::FACE_UP_DEFENSE,
        ))
        .unwrap();
    let revealed = layer.card_at(field).unwrap();
    assert_eq!(revealed.id, id);
    assert_eq!(revealed.code, 42);
    assert_eq!(revealed.position, Some(CardPositionFlags::FACE_UP_DEFENSE));

    layer
        .apply(&position_change(
            field,
            0,
            CardPositionFlags::FACE_UP_DEFENSE,
            CardPositionFlags::FACE_UP_ATTACK,
        ))
        .unwrap();
    layer
        .apply(&position_change(
            field,
            0,
            CardPositionFlags::FACE_UP_ATTACK,
            CardPositionFlags::FACE_DOWN_DEFENSE,
        ))
        .unwrap();
    let hidden = layer.card_at(field).unwrap();
    assert_eq!(hidden.id, id);
    assert_eq!(hidden.code, 42);
    assert_eq!(hidden.position, Some(CardPositionFlags::FACE_DOWN_DEFENSE));
}

#[test]
fn sets_move_draw_and_overlay_positions_from_the_messages() {
    let deck = address(Player::Player, CardLocation::Deck, 0);
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let field = address(Player::Player, CardLocation::MonsterZone, 0);
    let host = address(Player::Player, CardLocation::MonsterZone, 1);
    let mut layer = CardLayer::from_decks(&[42], &[]).unwrap();
    let id = layer.card_at(deck).unwrap().id;
    let host_id = layer.insert(host, 99).unwrap();
    layer.apply(&draw(Player::Player, &[42])).unwrap();
    assert_eq!(
        layer.card_at(hand).unwrap().position,
        Some(CardPositionFlags::FACE_DOWN_ATTACK | CardPositionFlags::FACE_DOWN_DEFENSE)
    );
    let mut destination = location(Some(field));
    destination.position = CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE);
    layer
        .apply(&overlay_move(location(Some(hand)), destination, 42))
        .unwrap();
    assert_eq!(
        layer.card_at(field).unwrap().position,
        Some(CardPositionFlags::FACE_DOWN_DEFENSE)
    );
    layer
        .apply(&overlay_move(
            location(Some(field)),
            overlay(Some(host), 0),
            42,
        ))
        .unwrap();
    assert_eq!(layer.material_at(host_id, 0).unwrap().id, id);
    assert_eq!(layer.material_at(host_id, 0).unwrap().position, None);
    let mut destination = location(Some(field));
    destination.position = CardLocationPosition::Position(CardPositionFlags::FACE_UP_DEFENSE);
    layer
        .apply(&overlay_move(overlay(Some(host), 0), destination, 42))
        .unwrap();
    assert_eq!(
        layer.card_at(field).unwrap().position,
        Some(CardPositionFlags::FACE_UP_DEFENSE)
    );
}

#[test]
fn summoning_notifications_update_appearance_without_replacing_card() {
    let field = address(Player::Opponent, CardLocation::MonsterZone, 5);
    let mut layer = CardLayer::default();
    let id = layer.insert(field, 0).unwrap();
    let mut info = location(Some(field));
    info.position = CardLocationPosition::Position(CardPositionFlags::FACE_DOWN_DEFENSE);
    layer
        .apply(&CoreMessage::SpecialSummoning(SummoningMessageData {
            card_code: 0,
            location: info,
        }))
        .unwrap();
    assert_eq!(layer.card_at(field).unwrap().code, 0);
    assert_eq!(
        layer.card_at(field).unwrap().position,
        Some(CardPositionFlags::FACE_DOWN_DEFENSE)
    );
    info.position = CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK);
    layer
        .apply(&CoreMessage::FlipSummoning(SummoningMessageData {
            card_code: 42,
            location: info,
        }))
        .unwrap();
    assert_eq!(layer.card_at(field).unwrap().id, id);
    assert_eq!(layer.card_at(field).unwrap().code, 42);
    assert_eq!(
        layer.card_at(field).unwrap().position,
        Some(CardPositionFlags::FACE_UP_ATTACK)
    );
    layer
        .apply(&CoreMessage::Summoning(SummoningMessageData {
            card_code: 42,
            location: info,
        }))
        .unwrap();
    assert_eq!(layer.card_at(field).unwrap().id, id);
}

#[test]
fn rejects_invalid_appearance_updates_atomically() {
    let field = address(Player::Player, CardLocation::MonsterZone, 0);
    let missing = address(Player::Player, CardLocation::MonsterZone, 1);
    let mut layer = CardLayer::default();
    layer.insert(field, 42).unwrap();
    let initial = layer.clone();
    for message in [
        position_change(
            field,
            99,
            CardPositionFlags::FACE_UP_ATTACK,
            CardPositionFlags::FACE_DOWN_DEFENSE,
        ),
        position_change(
            missing,
            42,
            CardPositionFlags::FACE_UP_ATTACK,
            CardPositionFlags::FACE_DOWN_DEFENSE,
        ),
        CoreMessage::SetCard(SetCardMessageData {
            card_code: 42,
            location: location(None),
        }),
        CoreMessage::FlipSummoning(SummoningMessageData {
            card_code: 42,
            location: overlay(Some(field), 0),
        }),
    ] {
        assert!(layer.apply(&message).is_err(), "{message:?}");
        assert_eq!(layer, initial);
    }
}

#[test]
fn draws_in_order_and_preserves_duplicate_ids() {
    let mut layer = CardLayer::from_decks(&[7, 8, 7, 9], &[]).unwrap();
    let original = layer.cards().to_vec();
    layer.apply(&draw(Player::Player, &[9, 7, 8])).unwrap();
    for (index, original_card) in original.iter().rev().take(3).enumerate() {
        assert_eq!(
            layer
                .card_at(address(Player::Player, CardLocation::Hand, index as u32))
                .unwrap()
                .id,
            original_card.id
        );
    }
    assert_eq!(codes(&layer, Player::Player, CardLocation::Deck), [7]);
    assert_eq!(codes(&layer, Player::Player, CardLocation::Hand), [9, 7, 8]);
    assert_eq!(
        layer
            .card_at(address(Player::Player, CardLocation::Deck, 0))
            .unwrap()
            .id,
        original[0].id
    );
    assert_ne!(original[0].id, original[2].id);
}

#[test]
fn opponent_draw_appends_to_their_hand() {
    let mut layer = CardLayer::from_decks(&[9], &[]).unwrap();
    let opponent = Player::Opponent;
    layer
        .insert(address(opponent, CardLocation::Deck, 0), 1)
        .unwrap();
    let id = layer
        .insert(address(opponent, CardLocation::Deck, 1), 2)
        .unwrap();
    layer
        .insert(address(opponent, CardLocation::Hand, 0), 3)
        .unwrap();
    layer.apply(&draw(opponent, &[2])).unwrap();
    assert_eq!(
        layer
            .card_at(address(opponent, CardLocation::Hand, 1))
            .unwrap()
            .id,
        id
    );
    assert_eq!(codes(&layer, opponent, CardLocation::Hand), [3, 2]);
    assert_eq!(codes(&layer, opponent, CardLocation::Deck), [1]);
    assert_eq!(codes(&layer, Player::Player, CardLocation::Deck), [9]);
}

#[test]
fn reorders_piles_and_compacts_both_ends() {
    let player = Player::Player;
    let deck = |sequence| address(player, CardLocation::Deck, sequence);
    let grave = |sequence| address(player, CardLocation::Graveyard, sequence);
    let mut layer = CardLayer::from_decks(&[1, 2, 3, 4], &[]).unwrap();
    layer.insert(grave(0), 10).unwrap();
    layer.insert(grave(1), 11).unwrap();

    let first = layer.card_at(deck(0)).unwrap().id;
    layer
        .apply(&movement(Some(deck(0)), Some(deck(3)), 1))
        .unwrap();
    assert_eq!(codes(&layer, player, CardLocation::Deck), [2, 3, 4, 1]);
    assert_eq!(layer.card_at(deck(3)).unwrap().id, first);
    layer
        .apply(&movement(Some(deck(3)), Some(deck(1)), 0))
        .unwrap();
    assert_eq!(codes(&layer, player, CardLocation::Deck), [2, 1, 3, 4]);
    layer
        .apply(&movement(Some(deck(2)), Some(grave(1)), 3))
        .unwrap();
    assert_eq!(codes(&layer, player, CardLocation::Deck), [2, 1, 4]);
    assert_eq!(codes(&layer, player, CardLocation::Graveyard), [10, 3, 11]);
}

#[test]
fn moves_between_fixed_zones_and_controllers() {
    let p = Player::Player;
    let o = Player::Opponent;
    let mut layer = CardLayer::default();
    let source = address(p, CardLocation::MonsterZone, 1);
    let untouched = address(p, CardLocation::MonsterZone, 4);
    let target = address(o, CardLocation::SpellTrapZone, 3);
    let id = layer.insert(source, 123).unwrap();
    let other_id = layer.insert(untouched, 456).unwrap();
    layer
        .apply(&movement(Some(source), Some(target), 123))
        .unwrap();
    assert!(layer.card_at(source).is_none());
    assert_eq!(layer.card_at(untouched).unwrap().id, other_id);
    assert_eq!(layer.card_at(target).unwrap().id, id);
}

#[test]
fn creates_reveals_and_removes_cards_without_reusing_ids() {
    let mut layer = CardLayer::default();
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let field = address(Player::Player, CardLocation::MonsterZone, 0);
    layer.apply(&movement(None, Some(hand), 0)).unwrap();
    let first = *layer.card_at(hand).unwrap();
    layer.apply(&movement(Some(hand), Some(field), 42)).unwrap();
    let revealed = layer.card_at(field).unwrap();
    assert_eq!(revealed.id, first.id);
    assert_eq!(revealed.code, 42);
    layer.apply(&movement(Some(field), None, 0)).unwrap();
    assert!(layer.card_at(field).is_none());
    layer.apply(&movement(None, Some(hand), 42)).unwrap();
    assert_ne!(layer.card_at(hand).unwrap().id, first.id);
}

#[test]
fn consecutive_moves_keep_identity_at_each_intermediate_address() {
    let mut layer = CardLayer::from_decks(&[42], &[]).unwrap();
    let deck = address(Player::Player, CardLocation::Deck, 0);
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let field = address(Player::Player, CardLocation::MonsterZone, 0);
    let id = layer.card_at(deck).unwrap().id;
    let messages = [
        movement(Some(deck), Some(hand), 42),
        movement(Some(hand), Some(field), 42),
    ];
    for (message, destination) in messages.iter().zip([hand, field]) {
        layer.apply(message).unwrap();
        assert_eq!(layer.card_at(destination).unwrap().id, id);
    }
}

#[test]
fn materials_follow_host_and_reindex_after_detachment() {
    let player = Player::Player;
    let host = address(player, CardLocation::MonsterZone, 1);
    let hand = address(player, CardLocation::Hand, 0);
    let moved_host = address(Player::Opponent, CardLocation::MonsterZone, 3);
    let grave = address(player, CardLocation::Graveyard, 0);
    let mut layer = CardLayer::default();
    let host_id = layer.insert(host, 100).unwrap();
    let first = layer.insert(hand, 7).unwrap();
    let second = layer
        .insert(address(player, CardLocation::Hand, 1), 7)
        .unwrap();

    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(host), 0),
            7,
        ))
        .unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(host), 1),
            7,
        ))
        .unwrap();
    assert_eq!(layer.material_at(host_id, 0).unwrap().id, first);
    assert_eq!(layer.material_at(host_id, 1).unwrap().id, second);
    assert_ne!(first, second);

    layer
        .apply(&movement(Some(host), Some(moved_host), 100))
        .unwrap();
    assert_eq!(layer.card_at(moved_host).unwrap().id, host_id);
    layer
        .apply(&overlay_move(
            overlay(Some(moved_host), 0),
            location(Some(grave)),
            7,
        ))
        .unwrap();
    assert_eq!(layer.card_at(grave).unwrap().id, first);
    assert_eq!(layer.material_at(host_id, 0).unwrap().id, second);
    assert!(layer.material_at(host_id, 1).is_none());
}

#[test]
fn materials_follow_host_into_grave_before_their_own_moves() {
    let host = address(Player::Player, CardLocation::MonsterZone, 0);
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let grave = address(Player::Player, CardLocation::Graveyard, 0);
    let mut layer = CardLayer::default();
    let host_id = layer.insert(host, 100).unwrap();
    let material_id = layer.insert(hand, 10).unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(host), 0),
            10,
        ))
        .unwrap();
    layer
        .apply(&movement(Some(host), Some(grave), 100))
        .unwrap();
    assert_eq!(layer.material_at(host_id, 0).unwrap().id, material_id);
    layer
        .apply(&overlay_move(
            overlay(Some(grave), 0),
            location(Some(address(Player::Player, CardLocation::Graveyard, 1))),
            10,
        ))
        .unwrap();
    assert_eq!(
        layer
            .card_at(address(Player::Player, CardLocation::Graveyard, 1))
            .unwrap()
            .id,
        material_id
    );
    assert!(layer.material_at(host_id, 0).is_none());
}

#[test]
fn overlay_destination_resolves_after_source_pile_compacts() {
    let player = Player::Player;
    let hand = |sequence| address(player, CardLocation::Hand, sequence);
    let mut layer = CardLayer::default();
    let material = layer.insert(hand(0), 10).unwrap();
    layer.insert(hand(1), 20).unwrap();
    let host = layer.insert(hand(2), 100).unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand(0))),
            overlay(Some(hand(1)), 0),
            10,
        ))
        .unwrap();
    assert_eq!(layer.card_at(hand(1)).unwrap().id, host);
    assert_eq!(layer.material_at(host, 0).unwrap().id, material);
    let grave = address(player, CardLocation::Graveyard, 0);
    layer
        .apply(&movement(Some(hand(0)), Some(grave), 20))
        .unwrap();
    assert_eq!(layer.card_at(hand(0)).unwrap().id, host);
    layer
        .apply(&overlay_move(
            overlay(Some(hand(0)), 0),
            location(Some(address(player, CardLocation::Graveyard, 1))),
            10,
        ))
        .unwrap();
    assert_eq!(
        layer
            .card_at(address(player, CardLocation::Graveyard, 1))
            .unwrap()
            .id,
        material
    );
}

#[test]
fn transfers_materials_between_hosts_without_changing_identity() {
    let player = Player::Player;
    let first_host = address(player, CardLocation::MonsterZone, 0);
    let second_host = address(player, CardLocation::MonsterZone, 1);
    let hand = address(player, CardLocation::Hand, 0);
    let mut layer = CardLayer::default();
    let first_id = layer.insert(first_host, 100).unwrap();
    let second_id = layer.insert(second_host, 200).unwrap();
    let material = layer.insert(hand, 10).unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(first_host), 0),
            10,
        ))
        .unwrap();
    layer
        .apply(&overlay_move(
            overlay(Some(first_host), 0),
            overlay(Some(second_host), 0),
            10,
        ))
        .unwrap();
    assert!(layer.material_at(first_id, 0).is_none());
    assert_eq!(layer.material_at(second_id, 0).unwrap().id, material);
}

#[test]
fn host_materials_transfer_before_the_host_becomes_a_material() {
    let player = Player::Player;
    let source = address(player, CardLocation::MonsterZone, 0);
    let target = address(player, CardLocation::MonsterZone, 1);
    let hand = address(player, CardLocation::Hand, 0);
    let mut layer = CardLayer::default();
    let source_id = layer.insert(source, 100).unwrap();
    let target_id = layer.insert(target, 200).unwrap();
    let material = layer.insert(hand, 10).unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(source), 0),
            10,
        ))
        .unwrap();
    let initial = layer.clone();
    assert!(
        layer
            .apply(&overlay_move(
                location(Some(source)),
                overlay(Some(target), 0),
                100,
            ))
            .is_err()
    );
    assert_eq!(layer, initial);
    layer
        .apply(&overlay_move(
            overlay(Some(source), 0),
            overlay(Some(target), 0),
            10,
        ))
        .unwrap();
    layer
        .apply(&overlay_move(
            location(Some(source)),
            overlay(Some(target), 1),
            100,
        ))
        .unwrap();
    assert_eq!(layer.material_at(target_id, 0).unwrap().id, material);
    assert_eq!(layer.material_at(target_id, 1).unwrap().id, source_id);
}

#[test]
fn material_sequences_are_full_width_and_reindex_after_detachment() {
    let host = address(Player::Player, CardLocation::MonsterZone, 0);
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let grave = address(Player::Player, CardLocation::Graveyard, 0);
    let mut layer = CardLayer::default();
    let host_id = layer.insert(host, 1000).unwrap();
    for sequence in 0..300 {
        layer.insert(hand, sequence + 1).unwrap();
        layer
            .apply(&overlay_move(
                location(Some(hand)),
                overlay(Some(host), sequence),
                sequence + 1,
            ))
            .unwrap();
    }
    let detached = layer.material_at(host_id, 256).unwrap().id;
    let shifted = layer.material_at(host_id, 257).unwrap().id;
    layer
        .apply(&overlay_move(
            overlay(Some(host), 256),
            location(Some(grave)),
            257,
        ))
        .unwrap();
    assert_eq!(layer.card_at(grave).unwrap().id, detached);
    assert_eq!(layer.material_at(host_id, 256).unwrap().id, shifted);
    assert!(layer.material_at(host_id, 299).is_none());
}

#[test]
fn handles_material_moves_after_a_host_disappears() {
    let host = address(Player::Player, CardLocation::MonsterZone, 0);
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let grave = address(Player::Player, CardLocation::Graveyard, 0);
    let mut layer = CardLayer::default();
    let host_id = layer.insert(host, 100).unwrap();
    let first = layer.insert(hand, 7).unwrap();
    let second = layer
        .insert(address(Player::Player, CardLocation::Hand, 1), 7)
        .unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(host), 0),
            7,
        ))
        .unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(host), 1),
            7,
        ))
        .unwrap();
    layer.apply(&movement(Some(host), None, 100)).unwrap();
    assert!(layer.card_at(host).is_none());
    layer
        .apply(&overlay_move(overlay(None, 0), location(Some(grave)), 7))
        .unwrap();
    assert_eq!(layer.card_at(grave).unwrap().id, first);
    assert_eq!(layer.material_at(host_id, 0).unwrap().id, second);
    layer
        .apply(&overlay_move(overlay(None, 0), location(None), 7))
        .unwrap();
    assert!(layer.material_at(host_id, 0).is_none());
}

#[test]
fn rejects_invalid_overlay_moves_atomically() {
    let host = address(Player::Player, CardLocation::MonsterZone, 0);
    let hand = address(Player::Player, CardLocation::Hand, 0);
    let mut layer = CardLayer::default();
    layer.insert(host, 100).unwrap();
    layer.insert(hand, 10).unwrap();
    let initial = layer.clone();
    let missing_host = address(Player::Player, CardLocation::MonsterZone, 1);
    let mut bad_position = overlay(Some(host), 0);
    bad_position.position = CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK);
    let mut bad_controller = overlay(Some(host), 0);
    bad_controller.controller = None;
    for message in [
        overlay_move(location(Some(hand)), overlay(Some(missing_host), 0), 10),
        overlay_move(location(Some(hand)), overlay(Some(host), 1), 10),
        overlay_move(overlay(Some(host), 0), location(Some(hand)), 10),
        overlay_move(location(Some(hand)), bad_position, 10),
        overlay_move(location(Some(hand)), bad_controller, 10),
        overlay_move(overlay(None, 0), location(Some(hand)), 10),
    ] {
        assert!(layer.apply(&message).is_err(), "{message:?}");
        assert_eq!(layer, initial);
    }
}

#[test]
fn ambiguous_disappeared_hosts_are_rejected() {
    let player = Player::Player;
    let first_host = address(player, CardLocation::MonsterZone, 0);
    let second_host = address(player, CardLocation::MonsterZone, 1);
    let hand = address(player, CardLocation::Hand, 0);
    let mut layer = CardLayer::default();
    layer.insert(first_host, 100).unwrap();
    layer.insert(second_host, 200).unwrap();
    layer.insert(hand, 10).unwrap();
    layer
        .insert(address(player, CardLocation::Hand, 1), 20)
        .unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(first_host), 0),
            10,
        ))
        .unwrap();
    layer
        .apply(&overlay_move(
            location(Some(hand)),
            overlay(Some(second_host), 0),
            20,
        ))
        .unwrap();
    layer.apply(&movement(Some(first_host), None, 100)).unwrap();
    layer
        .apply(&movement(Some(second_host), None, 200))
        .unwrap();
    let initial = layer.clone();
    assert!(
        layer
            .apply(&overlay_move(overlay(None, 0), location(Some(hand)), 10))
            .is_err()
    );
    assert_eq!(layer, initial);
}

#[test]
fn moves_full_width_sequences() {
    let mut layer = CardLayer::from_decks(&(0..300).collect::<Vec<_>>(), &[]).unwrap();
    let deck = |sequence| address(Player::Player, CardLocation::Deck, sequence);
    let id = layer.card_at(deck(299)).unwrap().id;
    layer
        .apply(&movement(Some(deck(299)), Some(deck(256)), 299))
        .unwrap();
    assert_eq!(layer.card_at(deck(256)).unwrap().id, id);
    assert_eq!(layer.card_at(deck(257)).unwrap().code, 256);
    assert_eq!(layer.card_at(deck(299)).unwrap().code, 298);
}

#[test]
fn invalid_messages_leave_all_state_unchanged() {
    let deck = |sequence| address(Player::Player, CardLocation::Deck, sequence);
    let field = address(Player::Player, CardLocation::MonsterZone, 0);
    let mut layer = CardLayer::from_decks(&[1, 2], &[]).unwrap();
    layer.insert(field, 3).unwrap();
    let initial = layer.clone();
    let mut overlay = movement(Some(deck(0)), Some(field), 1);
    if let CoreMessage::Move(message) = &mut overlay {
        message.current.location = CardLocationKind::Overlay {
            host: Some(CardLocation::MonsterZone),
        };
    }
    let mut missing_controller = movement(Some(deck(0)), Some(deck(1)), 1);
    if let CoreMessage::Move(message) = &mut missing_controller {
        message.current.controller = None;
    }
    let mut invalid_position = movement(Some(deck(0)), Some(deck(1)), 1);
    if let CoreMessage::Move(message) = &mut invalid_position {
        message.current.position = CardLocationPosition::OverlaySequence(0);
    }
    for message in [
        movement(None, None, 1),
        movement(Some(deck(9)), Some(field), 1),
        movement(Some(deck(0)), Some(field), 1),
        movement(Some(deck(0)), Some(deck(9)), 1),
        movement(Some(deck(0)), Some(deck(0)), 99),
        draw(Player::Player, &[2, 99]),
        draw(Player::Player, &[2, 1, 3]),
        overlay,
        missing_controller,
        invalid_position,
        CoreMessage::NewTurn,
    ] {
        assert!(layer.apply(&message).is_err(), "{message:?}");
        assert_eq!(layer, initial);
    }
    layer.next_id = u64::MAX;
    let exhausted = layer.clone();
    assert!(layer.apply(&movement(None, Some(deck(0)), 3)).is_err());
    assert_eq!(layer, exhausted);
}

#[test]
fn rejects_sparse_piles_atomically() {
    let mut layer = CardLayer::default();
    let deck = address(Player::Player, CardLocation::Deck, 8);
    layer.insert(deck, 1).unwrap();
    let initial = layer.clone();
    assert!(layer.apply(&draw(Player::Player, &[1])).is_err());
    assert!(layer.apply(&movement(Some(deck), None, 1)).is_err());
    assert_eq!(layer, initial);
}

#[test]
fn initializes_ordered_decks_and_distinguishes_copies() {
    let layer = CardLayer::from_decks(&[7, 8, 7], &[9, 9]).unwrap();
    let cards = layer.cards();
    assert_eq!(
        cards
            .iter()
            .map(|card| match card.placement {
                CardPlacement::Zone(address) => (address.location, address.sequence, card.code),
                CardPlacement::Overlay { .. } => panic!("Unexpected overlay material"),
            })
            .collect::<Vec<_>>(),
        [
            (CardLocation::Deck, 0, 7),
            (CardLocation::Deck, 1, 8),
            (CardLocation::Deck, 2, 7),
            (CardLocation::ExtraDeck, 0, 9),
            (CardLocation::ExtraDeck, 1, 9),
        ]
    );
    assert_ne!(cards[0].id, cards[2].id);
    assert_ne!(cards[3].id, cards[4].id);
    assert!(
        layer
            .card_at(address(Player::Opponent, CardLocation::Deck, 0))
            .is_none()
    );
}

#[test]
fn initializes_empty_decks() {
    assert!(CardLayer::from_decks(&[], &[]).unwrap().cards().is_empty());
}

#[test]
fn initializes_sequences_beyond_u8() {
    let deck: Vec<u32> = (1..=300).collect();
    let layer = CardLayer::from_decks(&deck, &[]).unwrap();
    let card = layer
        .card_at(address(Player::Player, CardLocation::Deck, 299))
        .unwrap();
    assert_eq!(card.code, 300);
    assert_eq!(card.id, CardId(299));
    assert_eq!(layer.cards().len(), 300);
}

#[test]
fn looks_up_cards_by_full_address() {
    let mut layer = CardLayer::default();
    for (player, location, sequence, code) in [
        (Player::Player, CardLocation::Deck, u32::MAX, 1),
        (Player::Player, CardLocation::Deck, 0, 2),
        (Player::Player, CardLocation::Deck, 256, 3),
        (Player::Player, CardLocation::ExtraDeck, 0, 4),
        (Player::Opponent, CardLocation::Deck, 0, 5),
    ] {
        layer
            .insert(address(player, location, sequence), code)
            .unwrap();
    }
    for (player, location, sequence, code) in [
        (Player::Player, CardLocation::Deck, u32::MAX, 1),
        (Player::Player, CardLocation::Deck, 0, 2),
        (Player::Player, CardLocation::Deck, 256, 3),
        (Player::Player, CardLocation::ExtraDeck, 0, 4),
        (Player::Opponent, CardLocation::Deck, 0, 5),
    ] {
        assert_eq!(
            layer
                .card_at(address(player, location, sequence))
                .unwrap()
                .code,
            code
        );
    }
    assert!(
        layer
            .card_at(address(Player::Opponent, CardLocation::Hand, 0))
            .is_none()
    );
}

#[test]
fn duplicate_addresses_and_exhaustion_do_not_change_the_list() {
    let mut layer = CardLayer::default();
    let first = address(Player::Player, CardLocation::Deck, 0);
    let second = address(Player::Player, CardLocation::Deck, 1);
    assert_eq!(layer.insert(first, 42).unwrap(), CardId(0));
    assert!(layer.insert(first, 99).is_err());
    assert_eq!(layer.insert(second, 42).unwrap(), CardId(1));
    layer.next_id = u64::MAX;
    let third = address(Player::Player, CardLocation::Deck, 2);
    assert!(layer.insert(third, 42).is_err());
    assert!(layer.card_at(third).is_none());
    assert_eq!(layer.cards().len(), 2);
}

#[test]
fn new_layer_resets_cards_and_allocation() {
    let first = address(Player::Player, CardLocation::Deck, 0);
    let mut old = CardLayer::default();
    assert_eq!(old.insert(first, 42).unwrap(), CardId(0));
    let mut new = CardLayer::default();
    assert!(new.card_at(first).is_none());
    assert_eq!(new.insert(first, 42).unwrap(), CardId(0));
}
