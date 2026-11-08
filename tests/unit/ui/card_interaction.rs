use super::*;
use crate::ocgcore::constants::Player;
use crate::ocgcore::messages::{
    IdleMessageData, SelectChainMessageData, SelectTributeMessageData, SelectUnselectMessageData,
};
use crate::ui::playback::use_playback;

fn address(controller: Player, location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller,
        location,
        sequence,
    }
}

fn card(controller: Player, location: CardLocation, sequence: u8, index: u8) -> CardData {
    CardData {
        card_code: 42,
        controller,
        location,
        sequence,
        action_index: Some(index),
        ..Default::default()
    }
}

#[test]
fn empty_prompts_offer_no_actions() {
    let actions = PendingInput::Idle(IdleMessageData::default()).card_actions(CardAddress {
        controller: Player::Player,
        location: CardLocation::Hand,
        sequence: 0,
    });
    assert_eq!(actions, CardActions::default());
}

#[test]
fn resolves_prompt_actions_by_full_address_and_preserves_indices() {
    let hand = CardLocation::Hand;
    let own = card(Player::Player, hand, 1, 7);
    let other_controller = card(Player::Opponent, hand, 1, 8);
    let other_sequence = card(Player::Player, hand, 2, 9);
    let last = card(Player::Player, hand, 1, 10);
    let effects = [own, other_controller, other_sequence, last];
    let actions = PendingInput::Idle(IdleMessageData {
        normal_summons: effects.to_vec(),
        monster_sets: vec![own],
        spell_trap_sets: vec![last],
        activatable_effects: effects.to_vec(),
        ..Default::default()
    })
    .card_actions(address(Player::Player, hand, 1));
    assert_eq!(actions.summon, Some(10));
    assert_eq!(actions.monster_set, Some(7));
    assert_eq!(actions.spell_trap_set, Some(10));
    assert_eq!(actions.effects, [own, last]);
    assert!(actions.prompted.is_none());
    let chain = PendingInput::Chain(SelectChainMessageData {
        player: 0,
        effects: effects.to_vec(),
    });
    assert_eq!(
        chain
            .card_actions(address(Player::Player, hand, 1))
            .prompted,
        Some(last)
    );
    assert_eq!(
        PendingInput::Idle(IdleMessageData {
            normal_summons: effects.to_vec(),
            ..Default::default()
        })
        .card_actions(address(Player::Player, hand, 257))
        .summon,
        None
    );
}

#[test]
fn resolves_selection_and_tribute_by_controller_and_response_order() {
    let field = CardLocation::MonsterZone;
    let own = card(Player::Player, field, 0, 3);
    let opponent = card(Player::Opponent, field, 0, 4);
    let mut selected = own;
    selected.is_selected = true;
    let selection = SelectUnselectMessageData {
        playerid: 0,
        finishable: true,
        cancelable: false,
        min_count: 1,
        max_count: 2,
        select_cards: vec![opponent],
        unselect_cards: vec![selected],
    };
    let tribute = SelectTributeMessageData {
        player: 0,
        is_cancelable: false,
        min_select: 1,
        max_select: 2,
        candidates: vec![opponent, own],
    };
    let prompts = PendingInput::Unselect(selection);
    let actions = prompts.card_actions(address(Player::Player, field, 0));
    assert_eq!(actions.selection, Some(1));
    assert!(actions.selectable && actions.selected);
    let opponent_actions = prompts.card_actions(address(Player::Opponent, field, 0));
    assert_eq!(opponent_actions.selection, Some(0));
    assert!(!opponent_actions.selected);
    let prompts = PendingInput::Tribute {
        message: tribute,
        selected: vec![3],
    };
    let actions = prompts.card_actions(address(Player::Player, field, 0));
    assert_eq!(actions.tribute, Some(3));
    assert!(actions.tribute_selectable && actions.tribute_selected);
    assert_eq!(actions.tribute_max, 2);
    let opponent_actions = prompts.card_actions(address(Player::Opponent, field, 0));
    assert_eq!(opponent_actions.tribute, Some(4));
    assert!(!opponent_actions.tribute_selected);
    let prompts = PendingInput::Chain(SelectChainMessageData {
        player: 0,
        effects: vec![own, selected],
    });
    assert_eq!(
        prompts
            .card_actions(address(Player::Player, field, 0))
            .prompted,
        Some(own)
    );
}

#[test]
fn menu_actions_are_suppressed_and_resolved_from_current_prompt() {
    use crate::duel::cards::CardTracker;

    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        use_playback();
        let mut view = use_context_provider(ViewState::default);
        let hand = address(Player::Player, CardLocation::Hand, 0);
        let mut layer = CardTracker::default();
        layer.insert(hand, 42).unwrap();
        state.cards.set(Some(layer));
        select_card(hand);
        state
            .pending_input
            .set(Some(PendingInput::Idle(IdleMessageData {
                normal_summons: vec![card(Player::Player, CardLocation::Hand, 0, 7)],
                ..Default::default()
            })));
        assert_eq!(
            current_menu_actions(&state, &view, hand).unwrap().summon,
            Some(7)
        );
        state
            .pending_input
            .set(Some(PendingInput::Idle(IdleMessageData {
                normal_summons: vec![card(Player::Player, CardLocation::Hand, 0, 9)],
                ..Default::default()
            })));
        assert_eq!(
            current_menu_actions(&state, &view, hand).unwrap().summon,
            Some(9)
        );
        view.effects_to_select_from
            .set(vec![card(Player::Player, CardLocation::Hand, 0, 1)]);
        assert!(current_menu_actions(&state, &view, hand).is_none());
        view.effects_to_select_from.set(vec![]);
        view.selected_card.set(None);
        assert!(current_menu_actions(&state, &view, hand).is_none());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
