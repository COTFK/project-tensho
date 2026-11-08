use super::events;
use super::{EffectPileModal, ExtraDeckModal, PileCard};
use crate::duel::cards::CardAddress;
use crate::duel::input::CardAction;
use crate::ocgcore::CardData;
use crate::ocgcore::constants::{CardLocation, Player};
use crate::ui::components::ModalSuppression;
use dioxus::core::{AttributeValue, ElementId, Mutation};
use dioxus::prelude::*;
use std::cell::RefCell;

#[derive(Clone, Copy)]
struct PileState {
    open: Signal<Option<CardLocation>>,
    cards: Signal<Vec<PileCard>>,
    available: Signal<Vec<CardData>>,
    suppress_actions: Signal<bool>,
    suppress_modal: Signal<bool>,
}

thread_local! {
    static STATE: RefCell<Option<PileState>> = const { RefCell::new(None) };
    static ACTIONS: RefCell<Vec<CardAction>> = const { RefCell::new(Vec::new()) };
}

fn app(location: CardLocation) -> Element {
    let cards = use_signal(move || {
        [42, 43]
            .into_iter()
            .enumerate()
            .map(|(index, card_code)| PileCard {
                data: CardData {
                    card_code,
                    controller: Player::Player,
                    location,
                    sequence: index as u8,
                    ..Default::default()
                },
                links: if index == 0 { vec![1, 4] } else { vec![2] },
            })
            .collect::<Vec<_>>()
    });
    let available = use_signal(move || vec![cards.peek()[0].data]);
    let open = use_signal(move || Some(location));
    let suppress_actions = use_signal(|| false);
    let suppress_modal = use_signal(|| false);
    let suppressed = use_memo(move || suppress_modal());
    use_context_provider(|| ModalSuppression(suppressed));
    STATE.with(|slot| {
        *slot.borrow_mut() = Some(PileState {
            open,
            cards,
            available,
            suppress_actions,
            suppress_modal,
        });
    });
    let onaction = move |action| ACTIONS.with(|actions| actions.borrow_mut().push(action));
    if location == CardLocation::ExtraDeck {
        rsx! {
            ExtraDeckModal {
                extra_deck: cards(),
                available_special_summons: available(),
                open_pile: open,
                suppress_actions: suppress_actions(),
                onaction,
            }
        }
    } else {
        rsx! {
            EffectPileModal {
                location,
                cards: cards(),
                cards_prompting_to_activate: available(),
                open_pile: open,
                suppress_actions: suppress_actions(),
                onaction,
            }
        }
    }
}

struct PileElements {
    cards: Vec<ElementId>,
    menus: Vec<ElementId>,
    actions: Vec<ElementId>,
    glow: ElementId,
    close: ElementId,
    modal: ElementId,
}

fn class_ids(edits: &[Mutation], class: &str) -> Vec<ElementId> {
    edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "class",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value.contains(class) => Some(*id),
            _ => None,
        })
        .collect()
}

fn class_at(edits: &[Mutation], element: ElementId) -> &str {
    edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "class",
                value: AttributeValue::Text(value),
                id,
                ..
            } if *id == element => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing class update for {element:?}: {edits:?}"))
}

fn mount(location: CardLocation) -> (VirtualDom, PileState, PileElements) {
    events::install();
    ACTIONS.with(|actions| actions.borrow_mut().clear());
    let mut dom = VirtualDom::new_with_props(app, location);
    let initial = dom.rebuild_to_vec();
    let cards = class_ids(&initial.edits, "w-[12vw]");
    let menus = class_ids(&initial.edits, "bg-black/60");
    let actions = class_ids(&initial.edits, "size-12");
    assert_eq!(cards.len(), 2, "{:?}", initial.edits);
    assert_eq!(menus.len(), 2, "{:?}", initial.edits);
    assert_eq!(actions.len(), 2, "{:?}", initial.edits);
    for id in &cards {
        let class = class_at(&initial.edits, *id);
        assert!(class.contains("border-2") && class.contains("border-transparent"));
        assert!(!class.contains("border-dashed"));
    }
    let glow = class_ids(&initial.edits, "bg-yellow-400");
    assert_eq!(glow.len(), 1);
    assert!(class_ids(&initial.edits, "bg-cyan-400").is_empty());
    assert!(
        menus
            .iter()
            .all(|id| class_at(&initial.edits, *id).contains("hidden"))
    );
    let images = initial
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "src",
                value: AttributeValue::Text(value),
                ..
            } => Some(value.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        images,
        [
            crate::utility::card_image_url(42),
            crate::utility::card_image_url(43)
        ]
    );
    let badges = initial
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "aria-label",
                value: AttributeValue::Text(value),
                ..
            } => Some(value.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        badges,
        [
            "Chain link 1; higher links resolve first",
            "Chain link 4; higher links resolve first",
            "Chain link 2; higher links resolve first",
        ]
    );
    let close = class_ids(&initial.edits, "bg-green-600/70");
    let modal = class_ids(&initial.edits, "transition-all");
    assert_eq!(close.len(), 1);
    assert_eq!(modal.len(), 1);
    dom.process_events();
    let state = STATE.with(|slot| slot.borrow().unwrap());
    (
        dom,
        state,
        PileElements {
            cards,
            menus,
            actions,
            glow: glow[0],
            close: close[0],
            modal: modal[0],
        },
    )
}

fn assert_retained(edits: &[Mutation]) {
    assert!(
        !edits
            .iter()
            .any(|edit| matches!(edit, Mutation::Remove { .. } | Mutation::ReplaceWith { .. })),
        "{edits:?}"
    );
}

#[test]
fn pile_actions_keep_payloads_and_distinct_close_behavior() {
    for location in [
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let (mut dom, mut state, elements) = mount(location);
        events::dispatch(&dom, "click", elements.cards[1]);
        let unavailable_selected = dom.render_immediate_to_vec();
        assert!(
            class_at(&unavailable_selected.edits, elements.cards[1]).contains("border-yellow-300")
        );
        dom.process_events();
        events::dispatch(&dom, "click", elements.cards[0]);
        let selected = dom.render_immediate_to_vec();
        assert!(!class_at(&selected.edits, elements.menus[0]).contains("hidden"));
        dom.process_events();
        events::dispatch(&dom, "click", elements.actions[0]);
        let acted = dom.render_immediate_to_vec();
        let address = CardAddress {
            controller: Player::Player,
            location,
            sequence: 0,
        };
        ACTIONS.with(|actions| {
            assert_eq!(
                *actions.borrow(),
                [if location == CardLocation::ExtraDeck {
                    CardAction::SpecialSummon(address)
                } else {
                    CardAction::Activate(address)
                }]
            );
        });
        if location == CardLocation::ExtraDeck {
            assert_eq!(*state.open.peek(), None);
            assert!(class_at(&acted.edits, elements.modal).contains("-translate-y-"));
            assert_retained(&acted.edits);
            dom.process_events();
            state.open.set(Some(location));
            let reopened = dom.render_immediate_to_vec();
            assert!(class_at(&reopened.edits, elements.menus[0]).contains("hidden"));
        } else {
            assert_eq!(*state.open.peek(), Some(location));
            assert!(class_at(&acted.edits, elements.menus[0]).contains("hidden"));
        }
        STATE.with(|slot| *slot.borrow_mut() = None);
    }
}

#[test]
fn explicit_close_clears_selection_but_external_close_retains_it() {
    for location in [
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let (mut dom, mut state, elements) = mount(location);
        events::dispatch(&dom, "click", elements.cards[0]);
        let selected = dom.render_immediate_to_vec();
        assert!(!class_at(&selected.edits, elements.menus[0]).contains("hidden"));
        dom.process_events();
        let cards = state.cards.peek().clone();
        state.open.set(None);
        state.cards.set(Vec::new());
        assert_retained(&dom.render_immediate_to_vec().edits);
        dom.process_events();
        state.cards.set(cards);
        state.open.set(Some(location));
        let reopened = dom.render_immediate_to_vec();
        assert!(
            !reopened.edits.iter().any(|edit| matches!(
                edit,
                Mutation::SetAttribute { name: "class", id, value: AttributeValue::Text(value), .. }
                    if *id == elements.menus[0] && value.contains("hidden")
            )),
            "{:?}",
            reopened.edits
        );
        dom.process_events();
        events::dispatch(&dom, "click", elements.close);
        assert_eq!(*state.open.peek(), None);
        assert_retained(&dom.render_immediate_to_vec().edits);
        dom.process_events();
        state.open.set(Some(location));
        let reopened = dom.render_immediate_to_vec();
        assert!(class_at(&reopened.edits, elements.menus[0]).contains("hidden"));
        STATE.with(|slot| *slot.borrow_mut() = None);
    }
}

#[test]
fn suppression_keeps_pile_rows_and_local_selection() {
    for location in [
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let (mut dom, mut state, elements) = mount(location);
        events::dispatch(&dom, "click", elements.cards[0]);
        dom.render_immediate_to_vec();
        dom.process_events();
        state.suppress_actions.set(true);
        let blocked = dom.render_immediate_to_vec();
        assert!(class_at(&blocked.edits, elements.menus[0]).contains("hidden"));
        assert!(class_at(&blocked.edits, elements.glow).contains("hidden"));
        assert!(!class_at(&blocked.edits, elements.glow).contains("bg-yellow-400"));
        assert_retained(&blocked.edits);
        dom.process_events();
        if location != CardLocation::ExtraDeck {
            events::dispatch(&dom, "click", elements.actions[0]);
            dom.render_immediate_to_vec();
            ACTIONS.with(|actions| assert!(actions.borrow().is_empty()));
            assert_eq!(*state.open.peek(), Some(location));
        }
        state.suppress_actions.set(false);
        let unblocked = dom.render_immediate_to_vec();
        assert!(!class_at(&unblocked.edits, elements.menus[0]).contains("hidden"));
        assert!(class_at(&unblocked.edits, elements.glow).contains("bg-yellow-400"));
        dom.process_events();
        state.suppress_modal.set(true);
        let closing = dom.render_immediate_to_vec();
        assert!(closing.edits.iter().any(|edit| matches!(
            edit, Mutation::SetAttribute { name: "inert", id, value: AttributeValue::Text(_), .. }
                if *id == elements.modal
        )));
        assert_retained(&closing.edits);
        dom.process_events();
        state.suppress_modal.set(false);
        let resumed = dom.render_immediate_to_vec();
        assert_retained(&resumed.edits);
        assert_eq!(*state.open.peek(), Some(location));
        dom.process_events();
        events::dispatch(&dom, "click", elements.actions[0]);
        dom.render_immediate_to_vec();
        ACTIONS.with(|actions| assert_eq!(actions.borrow().len(), 1));
        STATE.with(|slot| *slot.borrow_mut() = None);
    }
}

#[test]
fn eligibility_checks_keep_each_piles_existing_matching_rules() {
    for location in [
        CardLocation::ExtraDeck,
        CardLocation::Graveyard,
        CardLocation::Banishment,
    ] {
        let (mut dom, mut state, elements) = mount(location);
        state.cards.write()[0].data.sequence = 7;
        state.cards.write()[0].data.controller = Player::Opponent;
        state.available.set(vec![
            CardData {
                location: CardLocation::Hand,
                ..state.cards.peek()[0].data
            },
            CardData {
                sequence: 9,
                ..state.cards.peek()[0].data
            },
        ]);
        dom.render_immediate_to_vec();
        dom.process_events();
        events::dispatch(&dom, "click", elements.cards[0]);
        let unavailable = dom.render_immediate_to_vec();
        assert!(!unavailable.edits.iter().any(|edit| matches!(
            edit,
            Mutation::SetAttribute { name: "class", id, value: AttributeValue::Text(value), .. }
                if *id == elements.menus[0] && !value.contains("hidden")
        )));
        dom.process_events();
        let mut available = vec![
            CardData {
                location: CardLocation::Hand,
                ..Default::default()
            };
            256
        ];
        available.push(CardData {
            location,
            sequence: if location == CardLocation::ExtraDeck {
                7
            } else {
                0
            },
            ..Default::default()
        });
        state.available.set(available);
        let eligible = dom.render_immediate_to_vec();
        assert!(!class_at(&eligible.edits, elements.menus[0]).contains("hidden"));
        dom.process_events();
        events::dispatch(&dom, "click", elements.actions[0]);
        dom.render_immediate_to_vec();
        let address = CardAddress {
            controller: Player::Opponent,
            location,
            sequence: 7,
        };
        ACTIONS.with(|actions| {
            assert_eq!(
                *actions.borrow(),
                [if location == CardLocation::ExtraDeck {
                    CardAction::SpecialSummon(address)
                } else {
                    CardAction::Activate(address)
                }]
            );
        });
        STATE.with(|slot| *slot.borrow_mut() = None);
    }
}
