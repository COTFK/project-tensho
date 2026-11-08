use super::*;
use crate::duel::cards::CardAddress;
use crate::ocgcore::location_info::CardPositionFlags;
use crate::ocgcore::location_info::{CardLocationInfo, CardLocationKind, CardLocationPosition};
use crate::ocgcore::messages::move_card::MoveReason;
use crate::ocgcore::messages::{
    CoreMessage, MoveMessageData, PositionChangeMessageData, ShuffleCardsMessageData,
};

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
    let mut before = CardTracker::default();
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
    playback.begin(vec![], CardTracker::default());
    let first = playback.generation;
    playback.cancel();
    playback.begin(vec![], CardTracker::default());
    let second = playback.generation;
    playback.finish(first);
    assert!(playback.is_active());
    playback.finish(second);
    assert!(!playback.is_active());
    assert!(playback.source.is_none());
}

#[test]
fn position_changes_start_playback_without_changing_card_identity() {
    let field = address(CardLocation::MonsterZone, 0);
    let mut before = CardTracker::default();
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
    playback.begin(vec![], CardTracker::default());
    let old = playback.generation;
    playback.cancel();
    playback.finish(old);
    assert!(!playback.is_active());
}

#[test]
fn cancellation_invalidates_stale_completion() {
    let mut playback = CardAnimationState::default();
    playback.begin(vec![], CardTracker::default());
    let generation = playback.generation;
    playback.cancel();
    playback.begin(vec![], CardTracker::default());
    playback.finish(generation);
    assert!(playback.is_active());
}

#[test]
fn gather_phase_spreads_before_finishing() {
    let mut playback = CardAnimationState::default();
    playback.begin_gather(vec![], CardTracker::default());
    let generation = playback.generation;
    assert_eq!(playback.phase, AnimationPhase::HandShuffleGathering);
    playback.advance(
        generation.wrapping_add(1),
        AnimationPhase::HandShuffleGathering,
    );
    playback.advance(generation, AnimationPhase::HandShuffleStacking);
    assert_eq!(playback.phase, AnimationPhase::HandShuffleGathering);
    playback.advance(generation, AnimationPhase::HandShuffleGathering);
    assert_eq!(playback.phase, AnimationPhase::HandShuffleStacking);
    playback.advance(generation, AnimationPhase::HandShuffleGathering);
    assert_eq!(playback.phase, AnimationPhase::HandShuffleStacking);
    playback.advance(generation, AnimationPhase::HandShuffleStacking);
    assert_eq!(playback.phase, AnimationPhase::HandShuffleSpreading);
    playback.advance(generation, AnimationPhase::HandShuffleSpreading);
    assert_eq!(playback.phase, AnimationPhase::Idle);
    assert!(playback.source.is_none());
}

#[test]
fn movement_finishes_only_for_matching_phase_and_generation() {
    let mut playback = CardAnimationState::default();
    playback.begin(vec![], CardTracker::default());
    let generation = playback.generation;
    assert_eq!(playback.phase, AnimationPhase::Moving);
    playback.advance(generation, AnimationPhase::HandShuffleSpreading);
    playback.advance(generation.wrapping_add(1), AnimationPhase::Moving);
    assert_eq!(playback.phase, AnimationPhase::Moving);
    playback.advance(generation, AnimationPhase::Moving);
    assert_eq!(playback.phase, AnimationPhase::Idle);
    assert!(playback.source.is_none());
}

#[test]
fn cancelling_any_shuffle_phase_invalidates_its_timer() {
    for phase in [
        AnimationPhase::HandShuffleGathering,
        AnimationPhase::HandShuffleStacking,
        AnimationPhase::HandShuffleSpreading,
    ] {
        let mut playback = CardAnimationState::default();
        playback.begin_gather(vec![], CardTracker::default());
        let generation = playback.generation;
        if phase != AnimationPhase::HandShuffleGathering {
            playback.advance(generation, AnimationPhase::HandShuffleGathering);
        }
        if phase == AnimationPhase::HandShuffleSpreading {
            playback.advance(generation, AnimationPhase::HandShuffleStacking);
        }
        assert_eq!(playback.phase, phase);
        playback.cancel();
        playback.begin_gather(vec![], CardTracker::default());
        playback.advance(generation, phase);
        assert_eq!(playback.phase, AnimationPhase::HandShuffleGathering);
        assert!(playback.source.is_some());
    }
}

#[test]
fn hand_shuffle_is_detected_only_when_every_hand_card_is_replaced() {
    let hand = |sequence| address(CardLocation::Hand, sequence);
    let mut before = CardTracker::default();
    for (sequence, code) in [7, 8].into_iter().enumerate() {
        before.insert(hand(sequence as u32), code).unwrap();
    }
    let mut shuffled = before.clone();
    shuffled
        .apply(&CoreMessage::ShuffleHand(ShuffleCardsMessageData {
            player: Player::Player,
            card_codes: vec![8, 7],
        }))
        .unwrap();
    let new_ids: Vec<_> = shuffled.cards().iter().map(|card| card.id).collect();
    assert_eq!(reshuffled_hand(&before, &shuffled), Some(new_ids));
    shuffled.insert(hand(2), 9).unwrap();
    assert_eq!(reshuffled_hand(&before, &shuffled).unwrap().len(), 3);

    let mut drawn = before.clone();
    drawn.insert(hand(2), 9).unwrap();
    assert_eq!(reshuffled_hand(&before, &drawn), None);

    let mut single = CardTracker::default();
    single.insert(hand(0), 7).unwrap();
    let mut single_shuffled = single.clone();
    single_shuffled
        .apply(&CoreMessage::ShuffleHand(ShuffleCardsMessageData {
            player: Player::Player,
            card_codes: vec![7],
        }))
        .unwrap();
    assert_eq!(reshuffled_hand(&single, &single_shuffled), None);
}

#[test]
fn plans_shuffle_and_movement_from_the_same_batch() {
    let mut before = CardTracker::default();
    for (sequence, code) in [7, 8].into_iter().enumerate() {
        before
            .insert(address(CardLocation::Hand, sequence as u32), code)
            .unwrap();
    }
    let field = address(CardLocation::MonsterZone, 0);
    let field_id = before.insert(field, 9).unwrap();
    let shuffle = CoreMessage::ShuffleHand(ShuffleCardsMessageData {
        player: Player::Player,
        card_codes: vec![8, 7],
    });
    let position_change = CoreMessage::PositionChange(PositionChangeMessageData {
        card_code: 9,
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
        previous_position: CardPositionFlags::FACE_UP_ATTACK,
        current_position: CardPositionFlags::FACE_UP_DEFENSE,
    });
    let mut after = before.clone();
    after.apply(&shuffle).unwrap();
    after.apply(&position_change).unwrap();
    let hand_ids = reshuffled_hand(&before, &after).unwrap();
    let mut expected = vec![field_id];
    expected.extend(hand_ids);
    let plan = plan_animation(&[shuffle, position_change], &before, &after).unwrap();
    assert_eq!(plan, CardAnimation::HandShuffle(expected.clone()));
    let mut playback = CardAnimationState::default();
    plan.begin(&mut playback, before.clone());
    assert_eq!(playback.phase, AnimationPhase::HandShuffleGathering);
    assert_eq!(playback.source, Some(before));
    assert!(expected.iter().all(|id| playback.moving.contains(id)));
}

#[test]
fn plans_only_valid_local_shuffles_and_other_visible_movements() {
    let mut before = CardTracker::default();
    before.insert(address(CardLocation::Hand, 0), 7).unwrap();
    before.insert(address(CardLocation::Hand, 1), 8).unwrap();
    let field = address(CardLocation::MonsterZone, 0);
    let field_id = before.insert(field, 9).unwrap();
    let shuffle = CoreMessage::ShuffleHand(ShuffleCardsMessageData {
        player: Player::Player,
        card_codes: vec![8, 7],
    });
    let mut shuffled = before.clone();
    shuffled.apply(&shuffle).unwrap();
    assert_eq!(plan_animation(&[], &before, &shuffled), None);
    let opponent_shuffle = CoreMessage::ShuffleHand(ShuffleCardsMessageData {
        player: Player::Opponent,
        card_codes: vec![8, 7],
    });
    assert_eq!(
        plan_animation(&[opponent_shuffle], &before, &shuffled),
        None
    );
    let singleton_shuffle = CoreMessage::ShuffleHand(ShuffleCardsMessageData {
        player: Player::Player,
        card_codes: vec![7],
    });
    assert_eq!(
        plan_animation(&[singleton_shuffle], &before, &shuffled),
        None
    );
    assert_eq!(plan_animation(&[shuffle.clone()], &before, &before), None);

    let mut drawn = before.clone();
    drawn.insert(address(CardLocation::Hand, 2), 10).unwrap();
    assert_eq!(plan_animation(&[], &before, &drawn), None);

    let change = CoreMessage::PositionChange(PositionChangeMessageData {
        card_code: 9,
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 0,
        previous_position: CardPositionFlags::FACE_UP_ATTACK,
        current_position: CardPositionFlags::FACE_UP_DEFENSE,
    });
    let mut moved = before.clone();
    moved.apply(&change).unwrap();
    assert_eq!(
        plan_animation(&[shuffle, change], &before, &moved),
        Some(CardAnimation::Movement(vec![field_id]))
    );
}
