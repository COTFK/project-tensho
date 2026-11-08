use super::*;
use crate::duel::cards::CardTracker;
use crate::duel::controller::DuelController;
use crate::ocgcore::Response;
use crate::ocgcore::constants::Player;
use crate::ocgcore::messages::{
    IdleMessageData, SelectChainMessageData, SelectTributeMessageData, SelectUnselectMessageData,
};
use crate::state::{DuelState, apply_card_intent, current_card_action_options, handle_card_intent};
use crate::ui::playback::DuelPlayback;
use crate::ui::view_state::DuelViewState;

#[path = "events.rs"]
mod events;

fn address(controller: Player, location: CardLocation, sequence: u32) -> CardAddress {
    CardAddress {
        controller,
        location,
        sequence,
    }
}

fn card(controller: Player, location: CardLocation, sequence: u8, index: u8) -> DuelCardData {
    DuelCardData {
        card_code: 42,
        controller,
        location,
        sequence,
        action_index: Some(index),
        ..Default::default()
    }
}

fn image_app((data, image, passive): (DuelCardData, CardImage, bool)) -> Element {
    let pending = use_signal(|| None::<PendingInput>);
    use_context_provider(|| ReadSignal::new(pending));
    use_context_provider(|| {
        EventHandler::new(|_: CardIntent| -> () { panic!("Unexpected card action") })
    });
    rsx! {
        CardWithActions {
            card: data,
            address: address(data.controller, data.location, u32::from(data.sequence)),
            image,
            passive,
            selected: false,
            suppress: false,
        }
    }
}

#[test]
fn supplied_images_are_not_reinterpreted_and_keep_card_rotation() {
    use dioxus::core::{AttributeValue, Mutation};

    let image_data = DuelCardData {
        card_code: 54321,
        ..Default::default()
    };
    for location in [CardLocation::Hand, CardLocation::MonsterZone] {
        for passive in [false, true] {
            for position in [
                None,
                Some(BattlePosition::FaceUpAttack),
                Some(BattlePosition::FaceUpDefense),
                Some(BattlePosition::FaceDown),
                Some(BattlePosition::FaceDownAttack),
                Some(BattlePosition::FaceDownDefense),
            ] {
                let data = DuelCardData {
                    position,
                    ..card(Player::Local, location, 0, 0)
                };
                for image in [
                    CardImage::Face(image_data),
                    CardImage::Thumbnail(image_data),
                    CardImage::CardBack,
                    CardImage::ExtraDeckBack,
                ] {
                    let mut dom = VirtualDom::new_with_props(image_app, (data, image, passive));
                    let rendered = dom.rebuild_to_vec();
                    let urls = rendered
                        .edits
                        .iter()
                        .filter_map(|edit| match edit {
                            Mutation::SetAttribute {
                                name: "src",
                                value: AttributeValue::Text(value),
                                ..
                            } => Some(value.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(urls, [image.url()]);
                    let card = rendered
                        .edits
                        .iter()
                        .find_map(|edit| match edit {
                            Mutation::NewEventListener { name, id } if name == "click" => Some(*id),
                            _ => None,
                        })
                        .expect("Card should retain its click handler");
                    let class = rendered
                        .edits
                        .iter()
                        .find_map(|edit| match edit {
                            Mutation::SetAttribute {
                                name: "class",
                                value: AttributeValue::Text(value),
                                id,
                                ..
                            } if *id == card => Some(value.as_str()),
                            _ => None,
                        })
                        .expect("Card should retain its class");
                    let rotated = !passive
                        && location != CardLocation::Hand
                        && matches!(
                            position,
                            Some(BattlePosition::FaceUpDefense | BattlePosition::FaceDownDefense)
                        );
                    assert_eq!(class.contains("-rotate-90"), rotated);
                    assert_eq!(class.contains("w-full"), passive);
                    assert_eq!(
                        class.contains("w-[10vw]"),
                        !passive && location == CardLocation::Hand
                    );
                }
            }
        }
    }
}

#[test]
fn empty_prompts_offer_no_actions() {
    let actions = PendingInput::ActionSelection(IdleMessageData::default()).card_action_options(
        CardAddress {
            controller: Player::Local,
            location: CardLocation::Hand,
            sequence: 0,
        },
    );
    assert_eq!(actions, CardActionOptions::default());
}

#[test]
fn resolves_prompt_actions_by_full_address_and_preserves_indices() {
    let hand = CardLocation::Hand;
    let own = card(Player::Local, hand, 1, 7);
    let other_controller = card(Player::Opponent, hand, 1, 8);
    let other_sequence = card(Player::Local, hand, 2, 9);
    let last = card(Player::Local, hand, 1, 10);
    let effects = [own, other_controller, other_sequence, last];
    let actions = PendingInput::ActionSelection(IdleMessageData {
        normal_summons: effects.to_vec(),
        monster_sets: vec![own],
        spell_trap_sets: vec![last],
        activatable_effects: effects.to_vec(),
        ..Default::default()
    })
    .card_action_options(address(Player::Local, hand, 1));
    assert_eq!(actions.summon, Some(10));
    assert_eq!(actions.monster_set, Some(7));
    assert_eq!(actions.spell_trap_set, Some(10));
    assert_eq!(actions.effects, [own, last]);
    assert!(actions.prompted.is_none());
    let chain = PendingInput::ChainResponse(SelectChainMessageData {
        player: 0,
        effects: effects.to_vec(),
    });
    assert_eq!(
        chain
            .card_action_options(address(Player::Local, hand, 1))
            .prompted,
        Some(last)
    );
    assert_eq!(
        PendingInput::ActionSelection(IdleMessageData {
            normal_summons: effects.to_vec(),
            ..Default::default()
        })
        .card_action_options(address(Player::Local, hand, 257))
        .summon,
        None
    );
}

#[test]
fn resolves_selection_and_tribute_by_controller_and_response_order() {
    let field = CardLocation::MonsterZone;
    let own = card(Player::Local, field, 0, 3);
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
    let prompts = PendingInput::CardToggleSelection(selection);
    let actions = prompts.card_action_options(address(Player::Local, field, 0));
    assert_eq!(actions.selection, Some(1));
    assert!(actions.selectable && actions.selected);
    let opponent_actions = prompts.card_action_options(address(Player::Opponent, field, 0));
    assert_eq!(opponent_actions.selection, Some(0));
    assert!(!opponent_actions.selected);
    let prompts = PendingInput::TributeSelection {
        message: tribute,
        selected: vec![3],
    };
    let actions = prompts.card_action_options(address(Player::Local, field, 0));
    assert_eq!(actions.tribute, Some(3));
    assert!(actions.tribute_selectable && actions.tribute_selected);
    assert_eq!(actions.tribute_max, 2);
    let opponent_actions = prompts.card_action_options(address(Player::Opponent, field, 0));
    assert_eq!(opponent_actions.tribute, Some(4));
    assert!(!opponent_actions.tribute_selected);
    let prompts = PendingInput::ChainResponse(SelectChainMessageData {
        player: 0,
        effects: vec![own, selected],
    });
    assert_eq!(
        prompts
            .card_action_options(address(Player::Local, field, 0))
            .prompted,
        Some(own)
    );
}

#[test]
fn menu_actions_are_suppressed_and_resolved_from_current_prompt() {
    use crate::duel::cards::CardTracker;

    fn app() -> Element {
        let mut state = use_hook(DuelState::default);
        let playback = use_hook(DuelPlayback::default);
        let mut view = use_hook(DuelViewState::default);
        let controller = use_signal(|| None::<DuelController>);
        let hand = address(Player::Local, CardLocation::Hand, 0);
        let mut layer = CardTracker::default();
        layer.insert(hand, 42).unwrap();
        state.cards.set(Some(layer));
        handle_card_intent(
            state,
            view,
            playback,
            controller.into(),
            CardIntent::Select(hand),
        );
        state
            .pending_input
            .set(Some(PendingInput::ActionSelection(IdleMessageData {
                normal_summons: vec![card(Player::Local, CardLocation::Hand, 0, 7)],
                ..Default::default()
            })));
        assert_eq!(
            current_card_action_options(&state, &view, hand)
                .unwrap()
                .summon,
            Some(7)
        );
        state
            .pending_input
            .set(Some(PendingInput::ActionSelection(IdleMessageData {
                normal_summons: vec![card(Player::Local, CardLocation::Hand, 0, 9)],
                ..Default::default()
            })));
        assert_eq!(
            current_card_action_options(&state, &view, hand)
                .unwrap()
                .summon,
            Some(9)
        );
        view.effects_to_select_from
            .set(vec![card(Player::Local, CardLocation::Hand, 0, 1)]);
        assert!(current_card_action_options(&state, &view, hand).is_none());
        view.effects_to_select_from.set(vec![]);
        view.selected_card.set(None);
        assert!(current_card_action_options(&state, &view, hand).is_none());
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn a_rendered_menu_resolves_the_current_request_without_whole_app_context() {
    use dioxus::core::{AttributeValue, Mutation};
    use std::cell::RefCell;
    thread_local! {
        static STATE: RefCell<Option<(DuelState, DuelViewState)>> = const { RefCell::new(None) };
        static RESPONSES: RefCell<Vec<Response>> = const { RefCell::new(Vec::new()) };
    }
    fn app() -> Element {
        let mut state = use_hook(DuelState::default);
        let mut view = use_hook(DuelViewState::default);
        let hand = address(Player::Local, CardLocation::Hand, 0);
        use_hook(move || {
            let mut cards = CardTracker::default();
            let id = cards.insert(hand, 42).unwrap();
            state.cards.set(Some(cards));
            view.selected_card.set(Some(id));
            state
                .pending_input
                .set(Some(PendingInput::ActionSelection(IdleMessageData {
                    normal_summons: vec![card(Player::Local, CardLocation::Hand, 0, 7)],
                    ..Default::default()
                })));
        });
        use_context_provider(|| ReadSignal::new(state.pending_input));
        use_context_provider(|| {
            EventHandler::new(move |action: CardIntent| {
                if let Some(response) = apply_card_intent(&mut state, &mut view, action) {
                    RESPONSES.with(|responses| responses.borrow_mut().push(response));
                }
            })
        });
        STATE.with(|slot| *slot.borrow_mut() = Some((state, view)));
        let data = card(Player::Local, CardLocation::Hand, 0, 7);
        rsx! {
            CardWithActions {
                card: data,
                address: hand,
                image: CardImage::Face(data),
                selected: view.selected_card.read().is_some(),
                suppress: !view.effects_to_select_from.read().is_empty(),
            }
        }
    }
    events::install();
    RESPONSES.with(|responses| responses.borrow_mut().clear());
    let mut dom = VirtualDom::new(app);
    let initial = dom.rebuild_to_vec();
    dom.process_events();
    let summon = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "class",
                value: AttributeValue::Text(class),
                id,
                ..
            } if class.contains("border-cyan-500") => Some(*id),
            _ => None,
        })
        .expect("Summon action button should be mounted");
    let image = initial
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "click" && *id != summon => {
                Some(*id)
            }
            _ => None,
        })
        .expect("Card selection handler should be mounted");
    let (mut state, mut view) = STATE.with(|slot| slot.borrow().unwrap());
    let selected = *view.selected_card.peek();
    state
        .pending_input
        .set(Some(PendingInput::ActionSelection(IdleMessageData {
            normal_summons: vec![card(Player::Local, CardLocation::Hand, 0, 9)],
            ..Default::default()
        })));
    events::dispatch(&dom, "click", summon);
    RESPONSES
        .with(|responses| assert_eq!(*responses.borrow(), [Response::NormalSummon { index: 9 }]));
    assert!(view.selected_card.peek().is_none());

    view.selected_card.set(selected);
    view.effects_to_select_from
        .set(vec![card(Player::Local, CardLocation::Hand, 0, 1)]);
    events::dispatch(&dom, "click", summon);
    view.effects_to_select_from.clear();
    view.selected_card.set(None);
    events::dispatch(&dom, "click", summon);
    view.selected_card.set(selected);
    state.clear_pending_input();
    events::dispatch(&dom, "click", summon);
    view.selected_card.set(None);
    events::dispatch(&dom, "click", image);
    assert_eq!(*view.selected_card.peek(), selected);
    RESPONSES.with(|responses| {
        assert_eq!(*responses.borrow(), [Response::NormalSummon { index: 9 }]);
        responses.borrow_mut().clear();
    });
    STATE.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn card_intents_keep_activation_set_and_pile_response_encodings() {
    fn app() -> Element {
        let mut state = use_hook(DuelState::default);
        let mut view = use_hook(DuelViewState::default);
        let hand = address(Player::Local, CardLocation::Hand, 0);
        let field = address(Player::Local, CardLocation::MonsterZone, 0);
        let extra = address(Player::Local, CardLocation::ExtraDeck, 0);
        let grave = address(Player::Local, CardLocation::Graveyard, 0);
        let mut cards = CardTracker::default();
        let hand_id = cards.insert(hand, 42).unwrap();
        let field_id = cards.insert(field, 42).unwrap();
        cards.insert(extra, 42).unwrap();
        cards.insert(grave, 42).unwrap();
        state.cards.set(Some(cards));
        view.selected_card.set(Some(hand_id));
        state
            .pending_input
            .set(Some(PendingInput::ActionSelection(IdleMessageData {
                monster_sets: vec![card(Player::Local, CardLocation::Hand, 0, 5)],
                spell_trap_sets: vec![card(Player::Local, CardLocation::Hand, 0, 6)],
                activatable_effects: vec![
                    card(Player::Local, CardLocation::Hand, 0, 7),
                    card(Player::Local, CardLocation::Hand, 0, 9),
                ],
                special_summons: vec![
                    card(Player::Local, CardLocation::Hand, 0, 0),
                    card(Player::Local, CardLocation::ExtraDeck, 0, 1),
                ],
                ..Default::default()
            })));
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::Set(hand)),
            Some(Response::SetSpellTrap { index: 6 })
        );
        view.selected_card.set(Some(hand_id));
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::Activate(hand)),
            Some(Response::Activate { index: 9 })
        );
        view.open_pile.set(Some(CardLocation::ExtraDeck));
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::SpecialSummon(extra)),
            Some(Response::SpecialSummon { index: 1 })
        );
        view.open_pile.set(None);
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::SpecialSummon(extra)),
            None
        );

        view.selected_card.set(Some(field_id));
        let effects = vec![
            card(Player::Local, CardLocation::MonsterZone, 0, 2),
            card(Player::Local, CardLocation::MonsterZone, 0, 4),
        ];
        state
            .pending_input
            .set(Some(PendingInput::ActionSelection(IdleMessageData {
                activatable_effects: effects.clone(),
                ..Default::default()
            })));
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::Activate(field)),
            None
        );
        assert_eq!(*view.effects_to_select_from.peek(), effects);
        assert!(view.selected_card.peek().is_none());
        assert!(state.waiting_on_input());
        view.effects_to_select_from.clear();
        view.open_pile.set(Some(CardLocation::Graveyard));
        state
            .pending_input
            .set(Some(PendingInput::ChainResponse(SelectChainMessageData {
                player: 0,
                effects: vec![card(Player::Local, CardLocation::Graveyard, 0, 8)],
            })));
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::Activate(grave)),
            Some(Response::Chain { index: 8 })
        );
        state
            .pending_input
            .set(Some(PendingInput::EffectConfirmation(DuelCardData {
                action_index: None,
                ..card(Player::Local, CardLocation::Graveyard, 0, 0)
            })));
        assert_eq!(
            apply_card_intent(&mut state, &mut view, CardIntent::Activate(grave)),
            Some(Response::Yes)
        );
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}

#[test]
fn card_intents_toggle_tributes_and_respect_playback_blocking() {
    fn app() -> Element {
        let mut state = use_hook(DuelState::default);
        let mut view = use_hook(DuelViewState::default);
        let mut playback = use_hook(DuelPlayback::default);
        let controller = use_signal(|| None::<DuelController>);
        assert!(try_consume_context::<DuelState>().is_none());
        assert!(try_consume_context::<DuelViewState>().is_none());
        assert!(try_consume_context::<DuelPlayback>().is_none());
        let field = address(Player::Local, CardLocation::MonsterZone, 0);
        let mut cards = CardTracker::default();
        let id = cards.insert(field, 42).unwrap();
        state.cards.set(Some(cards));
        state
            .pending_input
            .set(Some(PendingInput::TributeSelection {
                message: SelectTributeMessageData {
                    player: 0,
                    is_cancelable: false,
                    min_select: 1,
                    max_select: 1,
                    candidates: vec![card(Player::Local, CardLocation::MonsterZone, 0, 7)],
                },
                selected: Vec::new(),
            }));
        handle_card_intent(
            state,
            view,
            playback,
            controller.into(),
            CardIntent::Select(field),
        );
        assert!(matches!(state.pending_input.peek().as_ref(),
                Some(PendingInput::TributeSelection { selected, .. }) if selected == &[7]));
        playback
            .animation
            .write()
            .begin(vec![id], CardTracker::default());
        handle_card_intent(
            state,
            view,
            playback,
            controller.into(),
            CardIntent::Select(field),
        );
        assert!(matches!(state.pending_input.peek().as_ref(),
                Some(PendingInput::TributeSelection { selected, .. }) if selected == &[7]));
        playback.cancel();
        handle_card_intent(
            state,
            view,
            playback,
            controller.into(),
            CardIntent::Select(field),
        );
        assert!(matches!(state.pending_input.peek().as_ref(),
                Some(PendingInput::TributeSelection { selected, .. }) if selected.is_empty()));
        state.clear_pending_input();
        handle_card_intent(
            state,
            view,
            playback,
            controller.into(),
            CardIntent::Select(field),
        );
        assert_eq!(*view.selected_card.peek(), Some(id));
        view.selected_card.set(None);
        rsx! {}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
}
