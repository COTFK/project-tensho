use super::*;
use crate::ocgcore::constants::Player;

fn card(index: Option<u8>) -> CardData {
    CardData {
        card_code: 42,
        controller: Player::Player,
        location: CardLocation::MonsterZone,
        sequence: 2,
        action_index: index,
        ..Default::default()
    }
}

#[test]
fn declines_preserve_response_encoding_and_cancelability() {
    for (input, response) in [
        (PendingInput::YesNo("Activate?".into()), Some(Response::No)),
        (PendingInput::EffectYesNo(card(None)), Some(Response::No)),
        (
            PendingInput::Chain(SelectChainMessageData {
                player: 0,
                effects: vec![card(Some(7))],
            }),
            Some(Response::PassPriority),
        ),
        (
            PendingInput::Chain(SelectChainMessageData {
                player: 0,
                effects: vec![card(None)],
            }),
            Some(Response::No),
        ),
        (
            PendingInput::Chain(SelectChainMessageData {
                player: 0,
                effects: Vec::new(),
            }),
            None,
        ),
        (PendingInput::Idle(IdleMessageData::default()), None),
    ] {
        assert_eq!(input.decline_response(), response);
    }
    for is_cancelable in [false, true] {
        let input = PendingInput::Tribute {
            message: SelectTributeMessageData {
                player: 0,
                is_cancelable,
                min_select: 1,
                max_select: 2,
                candidates: vec![card(Some(7))],
            },
            selected: vec![7],
        };
        assert_eq!(
            input.decline_response(),
            is_cancelable.then_some(Response::PassPriority)
        );
    }
}

#[test]
fn special_summons_keep_their_global_response_order_and_full_address() {
    let hand = CardData {
        location: CardLocation::Hand,
        ..card(Some(0))
    };
    let extra = CardData {
        location: CardLocation::ExtraDeck,
        ..card(Some(1))
    };
    let opponent = CardData {
        controller: Player::Opponent,
        ..extra
    };
    let input = PendingInput::Idle(IdleMessageData {
        special_summons: vec![hand, opponent, extra],
        ..Default::default()
    });
    assert_eq!(
        input
            .card_actions(CardAddress {
                controller: Player::Player,
                location: CardLocation::ExtraDeck,
                sequence: 2,
            })
            .special_summon,
        Some(2)
    );
    assert_eq!(
        input
            .card_actions(CardAddress {
                controller: Player::Player,
                location: CardLocation::ExtraDeck,
                sequence: 258,
            })
            .special_summon,
        None
    );
}

#[test]
fn effect_yes_no_exposes_only_its_prompted_card() {
    let card = card(None);
    let input = PendingInput::EffectYesNo(card);
    assert_eq!(input.prompted_effects(), &[card]);
    let address = CardAddress {
        controller: card.controller,
        location: card.location,
        sequence: u32::from(card.sequence),
    };
    assert_eq!(input.card_actions(address).prompted, Some(card));
    assert_eq!(
        input.card_actions(CardAddress {
            controller: Player::Opponent,
            ..address
        }),
        CardActions::default()
    );
    assert!(
        PendingInput::Idle(IdleMessageData::default())
            .prompted_effects()
            .is_empty()
    );
}
