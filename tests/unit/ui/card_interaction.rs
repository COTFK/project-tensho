use super::*;
use crate::ocgcore::constants::Player;

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
    let actions = PromptCards {
        normal_summons: &[],
        monster_sets: &[],
        spell_trap_sets: &[],
        activatable_effects: &[],
        prompted_effects: &[],
        selections: None,
        tributes: None,
        selected_tributes: &[],
    }
    .for_address(CardAddress {
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
    let actions = PromptCards {
        normal_summons: &effects,
        monster_sets: &[own],
        spell_trap_sets: &[last],
        activatable_effects: &effects,
        prompted_effects: &effects,
        selections: None,
        tributes: None,
        selected_tributes: &[],
    }
    .for_address(address(Player::Player, hand, 1));
    assert_eq!(actions.summon, Some(10));
    assert_eq!(actions.monster_set, Some(7));
    assert_eq!(actions.spell_trap_set, Some(10));
    assert_eq!(actions.effects, [own, last]);
    assert_eq!(actions.prompted, Some(last));
    assert_eq!(
        PromptCards {
            normal_summons: &effects,
            monster_sets: &[],
            spell_trap_sets: &[],
            activatable_effects: &[],
            prompted_effects: &[],
            selections: None,
            tributes: None,
            selected_tributes: &[],
        }
        .for_address(address(Player::Player, hand, 257))
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
    let prompts = PromptCards {
        normal_summons: &[],
        monster_sets: &[],
        spell_trap_sets: &[],
        activatable_effects: &[],
        prompted_effects: &[own, selected],
        selections: Some(&selection),
        tributes: Some(&tribute),
        selected_tributes: &[3],
    };
    let actions = prompts.for_address(address(Player::Player, field, 0));
    assert_eq!(actions.selection, Some(1));
    assert!(actions.selectable && actions.selected);
    assert_eq!(actions.tribute, Some(3));
    assert!(actions.tribute_selectable && actions.tribute_selected);
    assert_eq!(actions.tribute_max, 2);
    assert_eq!(actions.prompted, Some(own));
    let opponent_actions = prompts.for_address(address(Player::Opponent, field, 0));
    assert_eq!(opponent_actions.selection, Some(0));
    assert_eq!(opponent_actions.tribute, Some(4));
    assert!(!opponent_actions.selected && !opponent_actions.tribute_selected);
}

#[test]
fn menu_actions_are_suppressed_and_resolved_from_current_prompt() {
    use crate::ui::card_layer::CardLayer;

    fn app() -> Element {
        let mut state = use_context_provider(UIState::default);
        let hand = address(Player::Player, CardLocation::Hand, 0);
        let mut layer = CardLayer::default();
        layer.insert(hand, 42).unwrap();
        state.cards.set(Some(layer));
        select_card(hand);
        state
            .normal_summons
            .set(vec![card(Player::Player, CardLocation::Hand, 0, 7)]);
        assert_eq!(current_menu_actions(&state, hand).unwrap().summon, Some(7));
        state
            .normal_summons
            .set(vec![card(Player::Player, CardLocation::Hand, 0, 9)]);
        assert_eq!(current_menu_actions(&state, hand).unwrap().summon, Some(9));
        state
            .effects_to_select_from
            .set(vec![card(Player::Player, CardLocation::Hand, 0, 1)]);
        assert!(current_menu_actions(&state, hand).is_none());
        state.effects_to_select_from.set(vec![]);
        state.selected_card.set(None);
        assert!(current_menu_actions(&state, hand).is_none());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
