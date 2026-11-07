use super::*;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ocgcore::location_info::{
    CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
};
use crate::ocgcore::messages::ChainLinkMessageData;
use crate::ui::card_layer::{CardAddress, CardPlacement};

fn address(location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller: Player::Player,
        location,
        sequence,
    }
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
    let mut cards = CardLayer::default();
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
        playback.record(&message, &cards);
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
    assert_eq!(playback.links_for_card(first), vec![1]);
    assert_eq!(playback.links_for_card(second), vec![2]);
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
fn identifies_overlay_material_as_the_chain_source() {
    let mut cards = CardLayer::default();
    let host = cards
        .insert(address(CardLocation::MonsterZone, 0), 1)
        .unwrap();
    let material = cards
        .insert(address(CardLocation::Graveyard, 0), 2)
        .unwrap();
    cards
        .cards_mut()
        .iter_mut()
        .find(|card| card.id == material)
        .unwrap()
        .placement = CardPlacement::Overlay { host, sequence: 0 };
    let mut message = chaining(1, address(CardLocation::MonsterZone, 0));
    let CoreMessage::Chaining(data) = &mut message else {
        unreachable!()
    };
    data.location.location = CardLocationKind::Overlay {
        host: Some(CardLocation::MonsterZone),
    };
    data.location.position = CardLocationPosition::OverlaySequence(0);
    let mut playback = ChainPlayback::default();
    playback.record(&message, &cards);
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
    let mut cards = CardLayer::default();
    let card = cards
        .insert(address(CardLocation::MonsterZone, 0), 1)
        .unwrap();
    let mut playback = ChainPlayback::default();
    playback.record(&chaining(1, address(CardLocation::MonsterZone, 0)), &cards);
    playback.start();
    playback.advance(playback.generation);
    assert!(!playback.active);
    playback.record(&chaining(2, address(CardLocation::MonsterZone, 0)), &cards);
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
    assert_eq!(playback.links_for_card(card), vec![1, 2]);
    playback.advance(playback.generation);
    playback.record(&CoreMessage::ChainEnd, &cards);
    playback.start();
    assert!(playback.displayed.is_empty());
}

#[test]
fn cancelled_or_obsolete_timers_do_not_advance_a_new_chain() {
    let mut cards = CardLayer::default();
    cards.insert(address(CardLocation::Hand, 0), 1).unwrap();
    let mut playback = ChainPlayback::default();
    playback.record(&chaining(1, address(CardLocation::Hand, 0)), &cards);
    playback.record(&chaining(2, address(CardLocation::Hand, 0)), &cards);
    playback.start();
    let first = playback.generation;
    playback.advance(first);
    playback.advance(first);
    assert_eq!(playback.displayed.len(), 2);
    let old = playback.generation;
    playback.cancel();
    playback.record(&chaining(1, address(CardLocation::Hand, 0)), &cards);
    playback.start();
    playback.advance(old);
    assert!(playback.active);
    assert_eq!(playback.displayed.len(), 1);
}
