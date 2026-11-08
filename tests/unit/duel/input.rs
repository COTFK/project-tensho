use super::*;
use crate::ocgcore::constants::Player;

fn card(index: Option<u8>) -> DuelCardData {
    DuelCardData {
        card_code: 42,
        controller: Player::Local,
        location: CardLocation::MonsterZone,
        sequence: 2,
        action_index: index,
        ..Default::default()
    }
}

#[test]
fn declines_preserve_response_encoding_and_cancelability() {
    for (input, response) in [
        (
            PendingInput::Confirmation("Activate?".into()),
            Some(Response::No),
        ),
        (
            PendingInput::EffectConfirmation(card(None)),
            Some(Response::No),
        ),
        (
            PendingInput::ChainResponse(SelectChainMessageData {
                player: 0,
                effects: vec![card(Some(7))],
            }),
            Some(Response::PassPriority),
        ),
        (
            PendingInput::ChainResponse(SelectChainMessageData {
                player: 0,
                effects: vec![card(None)],
            }),
            Some(Response::No),
        ),
        (
            PendingInput::ChainResponse(SelectChainMessageData {
                player: 0,
                effects: Vec::new(),
            }),
            None,
        ),
        (
            PendingInput::ActionSelection(IdleMessageData::default()),
            None,
        ),
    ] {
        assert_eq!(input.decline_response(), response);
    }
    for is_cancelable in [false, true] {
        let input = PendingInput::TributeSelection {
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
    let hand = DuelCardData {
        location: CardLocation::Hand,
        ..card(Some(0))
    };
    let extra = DuelCardData {
        location: CardLocation::ExtraDeck,
        ..card(Some(1))
    };
    let opponent = DuelCardData {
        controller: Player::Opponent,
        ..extra
    };
    let input = PendingInput::ActionSelection(IdleMessageData {
        special_summons: vec![hand, opponent, extra],
        ..Default::default()
    });
    assert_eq!(
        input
            .card_action_options(CardAddress {
                controller: Player::Local,
                location: CardLocation::ExtraDeck,
                sequence: 2,
            })
            .special_summon,
        Some(2)
    );
    assert_eq!(
        input
            .card_action_options(CardAddress {
                controller: Player::Local,
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
    let input = PendingInput::EffectConfirmation(card);
    assert_eq!(input.prompted_effects(), &[card]);
    let address = CardAddress {
        controller: card.controller,
        location: card.location,
        sequence: u32::from(card.sequence),
    };
    assert_eq!(input.card_action_options(address).prompted, Some(card));
    assert_eq!(
        input.card_action_options(CardAddress {
            controller: Player::Opponent,
            ..address
        }),
        CardActionOptions::default()
    );
    assert!(
        PendingInput::ActionSelection(IdleMessageData::default())
            .prompted_effects()
            .is_empty()
    );
}
