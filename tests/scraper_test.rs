use felex::scraper::marketplace::fetch_listings;

#[tokio::test]
async fn test_fetch_listings() {
    let listings = fetch_listings("пшеница", "RU").await.unwrap();
    assert!(!listings.is_empty(), "Should return at least one listing");
    
    let first = &listings[0];
    assert!(!first.title.is_empty());
    assert!(first.price > 0.0);
    assert!(!first.url.is_empty());
}
