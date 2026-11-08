use super::*;
use crate::duel::cards::CardTracker;
use crate::duel::input::PendingInput;
use crate::ocgcore::CardData;
use crate::ocgcore::messages::{SelectCardMessageData, SortCardMessageData};
use crate::state::UIState;
use crate::ui::card_animation::CardAnimationState;
use crate::ui::components::ModalSuppression;
use crate::ui::playback::use_playback;
use dioxus::core::{AttributeValue, Mutation};
use std::cell::RefCell;

#[path = "events.rs"]
mod events;

#[path = "pile_modals.rs"]
mod pile_modals;

thread_local! {
    static STATE: RefCell<Option<UIState>> = const { RefCell::new(None) };
    static PLAYBACK: RefCell<Option<Signal<CardAnimationState>>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let state = use_hook(UIState::default);
    let playback = use_playback();
    let suppressed = use_memo(move || state.interaction_blocked(playback));
    use_context_provider(|| ModalSuppression(suppressed));
    STATE.with(|slot| *slot.borrow_mut() = Some(state));
    PLAYBACK.with(|slot| *slot.borrow_mut() = Some(playback.animation));
    let pending = (state.pending_input)();
    rsx! {
        CardSelector {
            selectables: match &pending { Some(PendingInput::Cards(message)) => Some(message.clone()), _ => None },
            onresponse: |_| -> () { panic!("Unexpected picker submission") },
        }
        SortCardSelector {
            sort_cards: match &pending { Some(PendingInput::Sort(message)) => Some(message.clone()), _ => None },
            onresponse: |_| -> () { panic!("Unexpected picker submission") },
        }
    }
}

#[test]
fn pile_rows_keep_card_order_and_chain_links_without_playback_state() {
    use crate::duel::cards::{CardAddress, CardPlacement};
    let mut tracking = CardTracker::default();
    let grave = CardAddress {
        controller: Player::Player,
        location: CardLocation::Graveyard,
        sequence: 0,
    };
    let first = tracking.insert(grave, 42).unwrap();
    let second = tracking
        .insert(
            CardAddress {
                sequence: 1,
                ..grave
            },
            42,
        )
        .unwrap();
    let extra = tracking
        .insert(
            CardAddress {
                location: CardLocation::ExtraDeck,
                ..grave
            },
            43,
        )
        .unwrap();
    for card in tracking.cards().to_vec() {
        let CardPlacement::Zone(address) = card.placement else {
            unreachable!()
        };
        tracking
            .update_queried_data(
                address,
                CardData {
                    card_code: card.code,
                    controller: address.controller,
                    location: address.location,
                    sequence: address.sequence as u8,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let links = [
        ChainLink {
            number: 1,
            card: Some(second),
        },
        ChainLink {
            number: 2,
            card: Some(first),
        },
        ChainLink {
            number: 3,
            card: Some(extra),
        },
        ChainLink {
            number: 4,
            card: Some(second),
        },
    ];
    let rows = pile_cards(Some(&tracking), CardLocation::Graveyard, &links);
    assert_eq!(
        rows.iter().map(|row| row.data.sequence).collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(rows[0].links, [2]);
    assert_eq!(rows[1].links, [1, 4]);
    assert_eq!(
        pile_cards(Some(&tracking), CardLocation::ExtraDeck, &links)[0].links,
        [3]
    );
    assert!(pile_cards(None, CardLocation::Graveyard, &links).is_empty());
}

#[test]
fn selection_and_sorting_lists_survive_cleared_prompts_while_closing() {
    for sorting in [false, true] {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        dom.process_events();
        let card = CardData {
            card_code: 42,
            ..Default::default()
        };
        STATE.with(|slot| {
            let mut state = slot.borrow().unwrap();
            state.pending_input.set(Some(if sorting {
                PendingInput::Sort(SortCardMessageData {
                    player: 0,
                    cards: vec![card],
                })
            } else {
                PendingInput::Cards(SelectCardMessageData {
                    min_select: 1,
                    max_select: 1,
                    cards: vec![card],
                })
            }));
        });
        let opened = dom.render_immediate_to_vec();
        assert_eq!(
            opened
                .edits
                .iter()
                .filter(|edit| matches!(edit,
                    Mutation::SetAttribute { name: "src", value: AttributeValue::Text(url), .. }
                        if *url == crate::utility::card_image_url(42)
                ))
                .count(),
            1,
            "{:?}",
            opened.edits
        );
        dom.process_events();

        STATE.with(|slot| {
            let mut state = slot.borrow().unwrap();
            state.clear_pending_input();
        });
        let closing = dom.render_immediate_to_vec();
        assert!(
            !closing
                .edits
                .iter()
                .any(|edit| matches!(edit, Mutation::Remove { .. } | Mutation::ReplaceWith { .. })),
            "{:?}",
            closing.edits
        );
        STATE.with(|slot| *slot.borrow_mut() = None);
        PLAYBACK.with(|slot| *slot.borrow_mut() = None);
    }
}

#[test]
fn playback_suppression_preserves_pending_requests_and_picker_selections() {
    events::install();
    for sorting in [false, true] {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        dom.process_events();
        let cards = [42, 43]
            .map(|card_code| CardData {
                card_code,
                ..Default::default()
            })
            .to_vec();
        let state = STATE.with(|slot| {
            let mut state = slot.borrow().unwrap();
            state.pending_input.set(Some(if sorting {
                PendingInput::Sort(SortCardMessageData { player: 0, cards })
            } else {
                PendingInput::Cards(SelectCardMessageData {
                    min_select: 2,
                    max_select: 2,
                    cards,
                })
            }));
            state
        });
        let request = state.pending_input.peek().clone();
        let opened = dom.render_immediate_to_vec();
        let done = opened
            .edits
            .iter()
            .find_map(|edit| match edit {
                Mutation::SetAttribute {
                    name: "disabled",
                    value: AttributeValue::Bool(true),
                    id,
                    ..
                } => Some(*id),
                _ => None,
            })
            .expect("Picker should require both cards");
        let card_ids = opened
            .edits
            .iter()
            .filter_map(|edit| match edit {
                Mutation::NewEventListener { name, id } if name == "click" && *id != done => {
                    Some(*id)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(card_ids.len(), 2);
        dom.process_events();
        events::dispatch(&dom, "click", card_ids[0]);
        let selected = dom.render_immediate_to_vec();
        assert!(
            selected.edits.iter().any(|edit| matches!(edit,
                Mutation::SetAttribute { name: "class", value: AttributeValue::Text(class), id, .. }
                    if *id == card_ids[0] && class.contains("border-yellow-300")
            )),
            "{:?}",
            selected.edits
        );
        dom.process_events();

        let mut playback = PLAYBACK.with(|slot| slot.borrow().unwrap());
        playback.write().begin(vec![], CardTracker::default());
        let suppressed = dom.render_immediate_to_vec();
        assert!(
            suppressed.edits.iter().any(|edit| matches!(
                edit,
                Mutation::SetAttribute {
                    name: "inert",
                    value: AttributeValue::Text(_),
                    ..
                }
            )),
            "{:?}",
            suppressed.edits
        );
        assert!(
            !suppressed
                .edits
                .iter()
                .any(|edit| matches!(edit, Mutation::Remove { .. } | Mutation::ReplaceWith { .. })),
            "{:?}",
            suppressed.edits
        );
        assert!(state.waiting_on_input());
        assert_eq!(*state.pending_input.peek(), request);
        dom.process_events();

        let generation = playback.peek().generation;
        playback.write().finish(generation);
        let resumed = dom.render_immediate_to_vec();
        assert!(
            resumed.edits.iter().any(|edit| matches!(
                edit,
                Mutation::SetAttribute {
                    name: "inert",
                    value: AttributeValue::None,
                    ..
                }
            )),
            "{:?}",
            resumed.edits
        );
        dom.process_events();
        events::dispatch(&dom, "click", card_ids[1]);
        let completed = dom.render_immediate_to_vec();
        assert!(completed.edits.iter().any(|edit| matches!(edit,
            Mutation::SetAttribute { name: "disabled", value: AttributeValue::Bool(false), id, .. }
                if *id == done
        )), "{:?}", completed.edits);
        assert!(state.waiting_on_input());
        assert_eq!(*state.pending_input.peek(), request);
        STATE.with(|slot| *slot.borrow_mut() = None);
        PLAYBACK.with(|slot| *slot.borrow_mut() = None);
    }
}
