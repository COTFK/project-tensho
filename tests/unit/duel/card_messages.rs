use crate::duel::cards::CardAddress;
use crate::ocgcore::constants::Player;
use crate::ocgcore::location_info::{
    CardLocationInfo, CardLocationKind, CardLocationPosition, CardPositionFlags,
};
use crate::ocgcore::messages::move_card::MoveReason;
use crate::ocgcore::messages::{CoreMessage, MoveMessageData};

pub(crate) fn location(address: Option<CardAddress>) -> CardLocationInfo {
    CardLocationInfo {
        controller: Some(address.map_or(Player::Player, |address| address.controller)),
        location: address.map_or(CardLocationKind::None, |address| {
            CardLocationKind::Known(address.location)
        }),
        sequence: address.map_or(0, |address| address.sequence),
        position: CardLocationPosition::Position(CardPositionFlags::FACE_UP_ATTACK),
    }
}

pub(crate) fn movement(
    from: Option<CardAddress>,
    to: Option<CardAddress>,
    code: u32,
) -> CoreMessage {
    CoreMessage::Move(MoveMessageData {
        card_code: code,
        previous: location(from),
        current: location(to),
        reason: MoveReason::EFFECT,
    })
}

pub(crate) fn overlay(host: Option<CardAddress>, sequence: u32) -> CardLocationInfo {
    CardLocationInfo {
        controller: host.map(|address| address.controller),
        location: CardLocationKind::Overlay {
            host: host.map(|address| address.location),
        },
        sequence: host.map_or(0, |address| address.sequence),
        position: CardLocationPosition::OverlaySequence(sequence),
    }
}

pub(crate) fn overlay_move(
    previous: CardLocationInfo,
    current: CardLocationInfo,
    code: u32,
) -> CoreMessage {
    CoreMessage::Move(MoveMessageData {
        card_code: code,
        previous,
        current,
        reason: MoveReason::XYZ | MoveReason::MATERIAL,
    })
}
