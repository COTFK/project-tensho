use dioxus::prelude::*;

use crate::ocgcore::UserResponse;
use crate::state::DuelState;
use crate::state::send_user_response;

#[component]
pub fn SlidingModal() -> Element {
    let state = use_context::<DuelState>();
    let mut card_ids = state.card_prompting_to_activate;

    let enabled = !card_ids.is_empty();

    rsx!(
        div {
            class: "absolute inset-0 w-[50vw] h-[8vh] bg-gray-500/25 mx-auto rounded-lg transition-all flex flex-row items-center px-4 duration-300 ease-in-out justify-between",
            class: if enabled { "top-[2vh] shadow-xl" } else { "-top-[8vh] shadow-none" },
            p {
                class: "text-white font-semibold",
                "A card or effect can be activated. Activate?"
            }
            button {
                class: "h-2/3 w-24 bg-red-600 rounded-lg font-semibold",
                onclick: move |_| {
                    send_user_response(UserResponse::RefusePromptedEffect);
                    card_ids.clear();
                },
                "Cancel"
            }
        }
    )
}
