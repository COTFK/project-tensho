use dioxus::prelude::*;

#[component]
pub fn Card(id: String) -> Element {
    let image_url = format!("https://images.ygoprodeck.com/images/cards/{id}.jpg");

    rsx!(div {
        image_rendering: "smooth",
        width: "12.5vw",
        aspect_ratio: "59/86",
        background_image: "url({image_url})",
        background_size: "cover",
    })
}
