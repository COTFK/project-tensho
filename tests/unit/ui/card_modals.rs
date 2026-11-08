use super::*;
use crate::ocgcore::CardData;
use crate::ocgcore::messages::{SelectCardMessageData, SortCardMessageData};
use dioxus::core::{AttributeValue, Mutation};
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<UIState>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let state = use_context_provider(UIState::default);
    STATE.with(|slot| *slot.borrow_mut() = Some(state));
    rsx! {
        CardSelector {}
        SortCardSelector {}
    }
}

#[test]
fn selection_and_sorting_lists_survive_cleared_prompts_while_closing() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.process_events();
    let card = CardData {
        card_code: 42,
        ..Default::default()
    };
    STATE.with(|slot| {
        let mut state = slot.borrow().unwrap();
        state.selectables.set(Some(SelectCardMessageData {
            min_select: 1,
            max_select: 1,
            cards: vec![card],
        }));
        state
            .sort_cards_to_select_from
            .set(Some(SortCardMessageData {
                player: 0,
                cards: vec![card],
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
        2,
        "{:?}",
        opened.edits
    );
    dom.process_events();

    STATE.with(|slot| {
        let mut state = slot.borrow().unwrap();
        state.selectables.set(None);
        state.sort_cards_to_select_from.set(None);
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
}
