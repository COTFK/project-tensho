mod card;
mod hand;
mod ocgcore;

use crate::hand::Hand;
use crate::ocgcore::load_ocgcore;
use dioxus::prelude::*;

static OCGCORE_WASM: Asset = asset!(
    "/assets/ocgcore.wasm",
    AssetOptions::builder()
        .with_hash_suffix(false)
        .into_asset_options()
);
static OCGCORE_JS: Asset = asset!(
    "/assets/ocgcore.js",
    AssetOptions::js().with_hash_suffix(false)
);
static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let _ = (OCGCORE_WASM, OCGCORE_JS);
    let ocgcore_status = use_resource(|| async move { load_ocgcore().await });
    let mut hand_contents = use_signal(|| Vec::<String>::new());
    let mut selected_card = use_signal(|| -1);

    let draw_card = move |_| {
        hand_contents.write().push(String::from("90681088"));
        println!("{hand_contents:#?}");
        println!("{selected_card:#?}");
    };

    let reset = move |_| {
        hand_contents.write().clear();
        selected_card.set(-1);
    };

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        main {
            class: "h-dvh w-dvw",
            button {
                class: "w-[10vw] aspect-[3/1] bg-[lightblue]",
                onclick: draw_card,
                "Draw"
            }
            button {
                class: "w-[10vw] aspect-[3/1] bg-[lightblue]",
                onclick: reset,
                "Reset"
            }
            div {
                {match ocgcore_status.read().as_ref() {
                    None => "loading ocgcore...".to_string(),
                    Some(Ok(())) => "ocgcore loaded".to_string(),
                    Some(Err(err)) => format!("ocgcore load error: {}", err),
                }}
            }
            Hand {
                cards: hand_contents,
                selected_card: selected_card,
            }
        }
    }
}
