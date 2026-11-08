use super::*;

#[test]
fn card_images_use_the_requested_face_thumbnail_or_back() {
    let card = DuelCardData {
        card_code: 12345,
        ..Default::default()
    };
    assert_eq!(CardImage::Face(card).url(), card_image_url(card.card_code));
    assert_eq!(
        CardImage::Thumbnail(card).url(),
        card_thumbnail_url(card.card_code)
    );
    assert_eq!(CardImage::CardBack.url(), CARD_BACK.to_string());
    assert_eq!(CardImage::ExtraDeckBack.url(), EXTRA_BACK.to_string());
}
