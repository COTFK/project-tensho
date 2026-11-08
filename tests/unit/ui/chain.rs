use super::*;
use crate::duel::cards::{CardAddress, CardId, CardTracker};
use crate::duel::controller::resolve_chain_event;
use crate::duel::test_messages::{location, overlay, overlay_move};
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{
    CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
};
use crate::ocgcore::messages::ChainLinkMessageData;
use crate::ocgcore::messages::CoreMessage;
use crate::ui::playback::use_playback;
use crate::ui::view_state::ViewState;

fn address(location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller: Player::Player,
        location,
        sequence,
    }
}

fn links_for_card(playback: &ChainPlayback, card: CardId) -> Vec<u32> {
    playback
        .displayed
        .iter()
        .filter_map(|link| (link.card == Some(card)).then_some(link.number))
        .collect()
}

fn chaining(number: u32, address: CardAddress) -> CoreMessage {
    let mut bytes = [
        33, 0, 0, 0, 70, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1, 0, 0, 0,
    ];
    bytes[33..37].copy_from_slice(&number.to_le_bytes());
    let mut message = CoreMessage::try_from(bytes.as_slice()).unwrap();
    let CoreMessage::Chaining(data) = &mut message else {
        panic!("Expected chain message");
    };
    data.location = CardLocationInfo {
        controller: Some(address.controller),
        location: CardLocationKind::Known(address.location),
        sequence: address.sequence,
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    };
    message
}

fn solved(number: u8) -> CoreMessage {
    CoreMessage::ChainSolved(ChainLinkMessageData { chain_link: number })
}

#[test]
fn plays_batched_chain_in_activation_and_resolution_order() {
    let mut cards = CardTracker::default();
    let first = cards
        .insert(address(CardLocation::MonsterZone, 0), 1)
        .unwrap();
    let second = cards
        .insert(address(CardLocation::Graveyard, 0), 2)
        .unwrap();
    let mut playback = ChainPlayback::default();
    for message in [
        chaining(1, address(CardLocation::MonsterZone, 0)),
        chaining(2, address(CardLocation::Graveyard, 0)),
        solved(2),
        solved(1),
        CoreMessage::ChainEnd,
    ] {
        playback.record(resolve_chain_event(&message, &cards).unwrap());
    }
    playback.start();
    assert_eq!(
        playback.displayed,
        [ChainLink {
            number: 1,
            card: Some(first)
        }]
    );
    assert!(playback.active);
    playback.advance(playback.generation);
    assert_eq!(
        playback.displayed,
        [
            ChainLink {
                number: 1,
                card: Some(first)
            },
            ChainLink {
                number: 2,
                card: Some(second)
            },
        ]
    );
    assert_eq!(links_for_card(&playback, first), vec![1]);
    assert_eq!(links_for_card(&playback, second), vec![2]);
    playback.advance(playback.generation);
    assert_eq!(
        playback.displayed,
        [ChainLink {
            number: 1,
            card: Some(first)
        }]
    );
    playback.advance(playback.generation);
    assert!(playback.displayed.is_empty());
    assert!(!playback.active);
}

#[test]
fn duel_failure_cancels_both_playbacks_and_invalidates_their_completions() {
    use crate::state::UIState;
    use crate::ui::card_animation::AnimationPhase;

    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        let playback = use_playback();
        let mut view = use_context_provider(ViewState::default);
        let mut animation = playback.animation;
        let mut chain = playback.chain;
        let mut cards = CardTracker::default();
        let card = cards.insert(address(CardLocation::Hand, 0), 1).unwrap();
        animation.write().begin(vec![card], cards.clone());
        for number in [1, 2] {
            chain.write().record(
                resolve_chain_event(&chaining(number, address(CardLocation::Hand, 0)), &cards)
                    .unwrap(),
            );
        }
        chain.write().start();
        let animation_generation = animation.peek().generation;
        let chain_generation = chain.peek().generation;
        assert!(animation.peek().is_active());
        assert!(chain.peek().active);
        assert!(chain.peek().has_pending_frames());

        state.fail("injected failure".into(), playback);

        assert_eq!(state.failure.peek().as_deref(), Some("injected failure"));
        assert!(state.interaction_blocked(playback));
        assert!(!animation.peek().is_active());
        assert!(animation.peek().moving.is_empty());
        assert!(animation.peek().source.is_none());
        assert!(!chain.peek().active);
        assert!(chain.peek().displayed.is_empty());
        assert!(!chain.peek().has_pending_frames());
        animation
            .write()
            .advance(animation_generation, AnimationPhase::Moving);
        chain.write().advance(chain_generation);
        chain.write().start();
        assert!(!animation.peek().is_active());
        assert!(!chain.peek().active);
        assert!(state.interaction_blocked(playback));

        state.reset_duel(&mut view, playback);
        animation.write().begin(vec![card], cards.clone());
        chain.write().record(
            resolve_chain_event(&chaining(3, address(CardLocation::Hand, 0)), &cards).unwrap(),
        );
        chain.write().start();
        animation
            .write()
            .advance(animation_generation, AnimationPhase::Moving);
        chain.write().advance(chain_generation);
        assert!(animation.peek().is_active());
        assert!(chain.peek().active);
        assert_eq!(links_for_card(&chain.peek(), card), vec![3]);
        assert!(state.interaction_blocked(playback));
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn identifies_overlay_material_as_the_chain_source() {
    let mut cards = CardTracker::default();
    cards
        .insert(address(CardLocation::MonsterZone, 0), 1)
        .unwrap();
    let material = cards
        .insert(address(CardLocation::Graveyard, 0), 2)
        .unwrap();
    cards
        .apply(&overlay_move(
            location(Some(address(CardLocation::Graveyard, 0))),
            overlay(Some(address(CardLocation::MonsterZone, 0)), 0),
            2,
        ))
        .unwrap();
    let mut message = chaining(1, address(CardLocation::MonsterZone, 0));
    let CoreMessage::Chaining(data) = &mut message else {
        unreachable!()
    };
    data.location.location = CardLocationKind::Overlay {
        host: Some(CardLocation::MonsterZone),
    };
    data.location.position = CardLocationPosition::OverlaySequence(0);
    let mut playback = ChainPlayback::default();
    playback.record(resolve_chain_event(&message, &cards).unwrap());
    playback.start();
    assert_eq!(
        playback.displayed,
        [ChainLink {
            number: 1,
            card: Some(material)
        }]
    );
}

#[test]
fn retains_all_links_from_same_card_and_clears_on_chain_end() {
    let mut cards = CardTracker::default();
    let card = cards
        .insert(address(CardLocation::MonsterZone, 0), 1)
        .unwrap();
    let mut playback = ChainPlayback::default();
    playback.record(
        resolve_chain_event(&chaining(1, address(CardLocation::MonsterZone, 0)), &cards).unwrap(),
    );
    playback.start();
    playback.advance(playback.generation);
    assert!(!playback.active);
    playback.record(
        resolve_chain_event(&chaining(2, address(CardLocation::MonsterZone, 0)), &cards).unwrap(),
    );
    playback.start();
    assert_eq!(
        playback.displayed,
        [
            ChainLink {
                number: 1,
                card: Some(card)
            },
            ChainLink {
                number: 2,
                card: Some(card)
            },
        ]
    );
    assert_eq!(links_for_card(&playback, card), vec![1, 2]);
    playback.advance(playback.generation);
    playback.record(resolve_chain_event(&CoreMessage::ChainEnd, &cards).unwrap());
    playback.start();
    assert!(playback.displayed.is_empty());
}

#[test]
fn cancelled_or_obsolete_timers_do_not_advance_a_new_chain() {
    let mut cards = CardTracker::default();
    cards.insert(address(CardLocation::Hand, 0), 1).unwrap();
    let mut playback = ChainPlayback::default();
    playback
        .record(resolve_chain_event(&chaining(1, address(CardLocation::Hand, 0)), &cards).unwrap());
    playback
        .record(resolve_chain_event(&chaining(2, address(CardLocation::Hand, 0)), &cards).unwrap());
    playback.start();
    let first = playback.generation;
    playback.advance(first);
    playback.advance(first);
    assert_eq!(playback.displayed.len(), 2);
    let old = playback.generation;
    playback.cancel();
    playback
        .record(resolve_chain_event(&chaining(1, address(CardLocation::Hand, 0)), &cards).unwrap());
    playback.start();
    playback.advance(old);
    assert!(playback.active);
    assert_eq!(playback.displayed.len(), 1);
}
