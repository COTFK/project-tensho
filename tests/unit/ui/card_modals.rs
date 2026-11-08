use super::*;
use crate::duel::cards::CardTracker;
use crate::duel::input::PendingInput;
use crate::ocgcore::CardData;
use crate::ocgcore::messages::{SelectCardMessageData, SortCardMessageData};
use crate::ui::card_animation::CardAnimationState;
use crate::ui::components::ModalSuppression;
use dioxus::core::{AttributeValue, Mutation};
use std::cell::RefCell;

#[path = "events.rs"]
mod events;

thread_local! {
    static STATE: RefCell<Option<UIState>> = const { RefCell::new(None) };
    static PLAYBACK: RefCell<Option<Signal<CardAnimationState>>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let state = use_context_provider(UIState::default);
    let playback = use_context_provider(|| Signal::new(CardAnimationState::default()));
    let suppressed = use_memo(move || state.interaction_blocked(Some(playback)));
    use_context_provider(|| ModalSuppression(suppressed));
    STATE.with(|slot| *slot.borrow_mut() = Some(state));
    PLAYBACK.with(|slot| *slot.borrow_mut() = Some(playback));
    rsx! {
        CardSelector {}
        SortCardSelector {}
    }
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
