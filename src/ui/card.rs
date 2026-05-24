use anyhow::anyhow;
use dioxus::prelude::*;

use super::components::ActivateButton;
use super::components::SelectableCard;
use crate::ocgcore::ActiveCard;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::CardLocation;
use crate::state::DuelState;
use crate::state::send_user_response;

#[component]
pub fn ActivatableCard(
    index: u8,
    card: Option<ActiveCard>,
    select_signal: WriteSignal<Option<u8>>,
) -> Element {
    let state = use_context::<DuelState>();
    let cards_prompting_to_activate = state.card_prompting_to_activate;

    let card = match card {
        Some(card) => card,
        None => return Err(anyhow!("Found empty activatable card").into()),
    };

    let prompted_card = cards_prompting_to_activate()
        .iter()
        .find(|card| card.location == CardLocation::Graveyard && card.sequence == index)
        .copied();

    let prompted = prompted_card.is_some();
    let chain_option = prompted_card.and_then(|card| card.chain_option);

    rsx!(
        div {
            class: "relative m-2 h-min",
            div {
                class: "absolute -inset-[2px] rounded-[4px] bg-yellow-400 blur-[2px] mix-blend-screen pointer-events-none -z-10",
                class: if !prompted {"hidden"},
            }
            SelectableCard {
                card_code: card.card_code,
                value_to_set: index,
                select_signal: select_signal
            }
            div {
                class: "absolute inset-1 border-5 border-yellow-300/50 blur-[2px] mix-blend-screen pointer-events-none animate-pulse z-20",
                class: if !prompted {"hidden"},
            }
            div {
                class: "absolute z-30 flex flex-col items-center justify-center w-min left-1/2 -translate-x-[50%] -translate-y-[128px] bg-black/60 px-8 py-1",
                class: if (select_signal() != Some(index)) || !prompted {"hidden"},
                style: "mask_image: linear-gradient(to right, transparent 0%, black 10%, black 90%, transparent 100%); -webkit-mask-image: linear-gradient(to right, transparent 0%, black 30%, black 70%, transparent 100%);",
                ActivateButton {
                    onclick: move |_| {
                        if prompted {
                            if let Some(chain_option) = chain_option {
                                send_user_response(UserResponse::Chain { sequence: chain_option });
                            } else {
                                send_user_response(UserResponse::Yes);
                            }

                            // if activatable {
                            //     send_user_response(UserResponse::Activate { sequence: activatable_eff_index as u8 });
                            // }

                            select_signal.set(None);
                        }
                    }
                }
            }
        }
    )
}
