use felex::agent::filter::score_listings;
use felex::scraper::marketplace::Listing;

#[tokio::test]
async fn test_ai_filter_fallback() {
    let mock_listings = vec![
        Listing {
            title: "Пшеница фуражная".to_string(),
            price: 10000.0,
            url: "http://example.com/1".to_string(),
        },
        Listing {
            title: "Кукуруза".to_string(),
            price: 12000.0,
            url: "http://example.com/2".to_string(),
        },
    ];

    let scores = score_listings("Пшеница", &mock_listings).await.unwrap();
    assert_eq!(scores.len(), 2);
    // Even if ollama is running or not, the test shouldn't panic, 
    // and should return 2 scores (likely 10 and 10 in fallback mode).
    assert!(scores[0] > 0);
    assert!(scores[1] > 0);
}
