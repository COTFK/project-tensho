use super::*;

#[test]
fn card_code_query_uses_repeated_padded_ids() {
    let request = reqwest::Client::new()
        .get("https://example.com/v1/cards")
        .query(&courtier_card_code_query(&[2526224, 57554544]))
        .build()
        .unwrap();
    assert_eq!(request.url().query(), Some("ids=02526224&ids=57554544"));
}

#[test]
fn card_code_batches_obey_api_limits() {
    assert_eq!(courtier_card_code_batches(&[]).count(), 0);
    let codes: Vec<_> = (0..201).collect();
    let batches: Vec<_> = courtier_card_code_batches(&codes).collect();
    assert_eq!(
        batches.iter().map(|batch| batch.len()).collect::<Vec<_>>(),
        [100, 100, 1]
    );
    assert_eq!(batches.concat(), codes);
}

#[test]
fn image_urls_use_image_routes() {
    assert_eq!(
        card_image_url(2526224),
        format!("{COURTIER_API_URL}/v1/images/02526224")
    );
    assert_eq!(
        card_thumbnail_url(2526224),
        format!("{COURTIER_API_URL}/v1/images/02526224/thumbnail")
    );
}
