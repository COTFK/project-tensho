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
    let mut playback = CardPlayback::default();
    playback.set_enabled(true);
    playback.begin(vec![], CardLayer::default(), Default::default());
    let first = playback.generation;
    playback.set_enabled(false);
    playback.set_enabled(true);
    playback.begin(vec![], CardLayer::default(), Default::default());
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
fn resize_cancels_playback_and_blocks_until_layout_is_ready() {
    let mut playback = CardPlayback {
        enabled: true,
        reduced_motion: false,
        ..Default::default()
    };
    playback.begin(vec![], CardLayer::default(), Default::default());
    let old = playback.generation;
    playback.resize();
    assert!(playback.layout_pending);
    assert!(playback.blocked());
    assert!(!playback.active);
    playback.finish(old);
    assert!(playback.blocked());
    playback.layout_pending = false;
    assert!(!playback.blocked());
}

#[test]
fn reduced_motion_and_restart_cancel_stale_playback() {
    let mut playback = CardPlayback {
        enabled: true,
        reduced_motion: false,
        ..Default::default()
    };
    playback.begin(vec![], CardLayer::default(), Default::default());
    let old = playback.generation;
    playback.set_reduced_motion(true);
    assert!(!playback.blocked());
    assert!(playback.reduced_motion);
    playback.set_reduced_motion(false);
    playback.begin(vec![], CardLayer::default(), Default::default());
    playback.restart();
    playback.finish(old);
    assert!(playback.restarting);
    assert!(playback.blocked());
}

#[test]
fn tracking_failure_stays_blocked_even_if_preview_is_disabled() {
    let mut playback = CardPlayback {
        enabled: true,
        reduced_motion: false,
        ..Default::default()
    };
    playback.begin(vec![], CardLayer::default(), Default::default());
    let generation = playback.generation;
    playback.fail("diverged".into());
    playback.set_enabled(false);
    playback.finish(generation);
    assert_eq!(playback.failure.as_deref(), Some("diverged"));
    assert!(playback.blocked());
    assert!(playback.source.is_none());
}
