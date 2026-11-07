use super::*;
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::location_info::{CardLocationInfo, CardLocationKind, CardLocationPosition};
use crate::ocgcore::messages::move_card::MoveReason;
use crate::ocgcore::messages::{CoreMessage, MoveMessageData, PositionChangeMessageData};
use crate::ui::card_layer::CardAddress;

fn address(location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller: Player::Player,
        location,
        sequence,
    }
}

fn location(address: CardAddress) -> CardLocationInfo {
    CardLocationInfo {
        controller: Some(address.controller),
        location: CardLocationKind::Known(address.location),
        sequence: address.sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    }
}

#[test]
fn groups_multiple_movements_and_ignores_new_or_hidden_cards() {
    let mut before = CardLayer::default();
    let first = before.insert(address(CardLocation::Hand, 0), 1).unwrap();
    let second = before.insert(address(CardLocation::Hand, 1), 2).unwrap();
    let mut after = before.clone();
    for (code, destination) in [(1, 0), (2, 1)] {
        after
            .apply(&CoreMessage::Move(MoveMessageData {
                card_code: code,
                previous: location(address(CardLocation::Hand, 0)),
                current: location(address(CardLocation::MonsterZone, destination)),
                reason: MoveReason::EFFECT,
            }))
            .unwrap();
    }
    after.insert(address(CardLocation::Deck, 0), 3).unwrap();
    assert_eq!(movements(&before, &after), vec![first, second]);
}

#[test]
fn completion_and_cancellation_ignore_stale_generations() {
    let mut playback = CardAnimationState::default();
    playback.begin(vec![], CardLayer::default());
    let first = playback.generation;
    playback.cancel();
    playback.begin(vec![], CardLayer::default());
    let second = playback.generation;
    playback.finish(first);
    assert!(playback.active);
    playback.finish(second);
    assert!(!playback.active);
    assert!(playback.source.is_none());
}

#[test]
fn position_changes_start_playback_without_changing_card_identity() {
    let field = address(CardLocation::MonsterZone, 0);
    let mut before = CardLayer::default();
    let id = before.insert(field, 1).unwrap();
    let mut after = before.clone();
    after
        .apply(&CoreMessage::PositionChange(PositionChangeMessageData {
            card_code: 1,
            controller: Player::Player,
            location: CardLocation::MonsterZone,
            sequence: 0,
            previous_position: CardPositionFlags::FACE_UP_ATTACK,
            current_position: CardPositionFlags::FACE_UP_DEFENSE,
        }))
        .unwrap();
    assert_eq!(movements(&before, &after), vec![id]);
    assert_eq!(after.card_at(field).unwrap().id, id);
}

#[test]
fn cancellation_stops_stale_playback() {
    let mut playback = CardAnimationState::default();
    playback.begin(vec![], CardLayer::default());
    let old = playback.generation;
    playback.cancel();
    playback.finish(old);
    assert!(!playback.active);
}

#[test]
fn cancellation_invalidates_stale_completion() {
    let mut playback = CardAnimationState::default();
    playback.begin(vec![], CardLayer::default());
    let generation = playback.generation;
    playback.cancel();
    playback.begin(vec![], CardLayer::default());
    playback.finish(generation);
    assert!(playback.active);
}
