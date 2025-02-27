use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;

use hft_engine::gateways::quote::QuoteGateway;
use hft_engine::types::Quote;
use hft_engine::error::{HftError, GatewayError};
use crate::mocks::mock_venue::{MockVenue, MockVenueConfig};

#[tokio::test]
async fn test_quote_gateway_add_venue() {
    // Create channels
    let (quote_tx, _quote_rx) = mpsc::channel(100);

    // Create gateway
    let gateway = QuoteGateway::new(quote_tx);

    // Create mock venue
    let venue = Arc::new(MockVenue::new("MOCK", MockVenueConfig::default()));

    // Add venue to gateway
    gateway.add_venue(venue.clone()).await;

    // Check that venue was added
    let venues = gateway.venues.read().await;
    assert_eq!(venues.len(), 1);
}

#[tokio::test]
async fn test_quote_gateway_remove_venue() {
    // Create channels
    let (quote_tx, _quote_rx) = mpsc::channel(100);

    // Create gateway
    let gateway = QuoteGateway::new(quote_tx);

    // Create mock venues
    let venue1 = Arc::new(MockVenue::new("MOCK1", MockVenueConfig::default()));
    let venue2 = Arc::new(MockVenue::new("MOCK2", MockVenueConfig::default()));

    // Add venues to gateway
    gateway.add_venue(venue1.clone()).await;
    gateway.add_venue(venue2.clone()).await;

    // Check that venues were added
    let venues = gateway.venues.read().await;
    assert_eq!(venues.len(), 2);

    // Remove one venue
    let result = gateway.remove_venue("MOCK1").await;
    assert!(result.is_ok());

    // Check that venue was removed
    let venues = gateway.venues.read().await;
    assert_eq!(venues.len(), 1);

    // Try to remove a venue that doesn't exist
    let result = gateway.remove_venue("NONEXISTENT").await;
    assert!(result.is_err());

    if let Err(HftError::Gateway(GatewayError::VenueNotFound(name))) = result {
        assert_eq!(name, "NONEXISTENT");
    } else {
        panic!("Expected VenueNotFound error, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_quote_gateway_subscribe() {
    // Create channels
    let (quote_tx, mut quote_rx) = mpsc::channel(100);

    // Create gateway
    let gateway = QuoteGateway::new(quote_tx);

    // Try to subscribe with no venues
    let result = gateway.subscribe(vec!["BTCUSDT".to_string()]).await;
    assert!(result.is_err());

    if let Err(HftError::Gateway(GatewayError::NoVenuesConfigured)) = result {
        // Expected error
    } else {
        panic!("Expected NoVenuesConfigured error, got: {:?}", result);
    }

    // Add a venue
    let venue = Arc::new(MockVenue::new("MOCK", MockVenueConfig::default())
        .with_quote_sender(gateway.quote_tx.clone()));

    gateway.add_venue(venue.clone()).await;

    // Subscribe to empty symbol list
    let result = gateway.subscribe(vec![]).await;
    assert!(result.is_err());

    if let Err(HftError::Gateway(GatewayError::InvalidSymbol(msg))) = result {
        assert!(msg.contains("Empty"));
    } else {
        panic!("Expected InvalidSymbol error, got: {:?}", result);
    }

    // Subscribe to valid symbols
    let symbols = vec!["BTCUSDT".to_string()];
    let result = gateway.subscribe(symbols).await;
    assert!(result.is_ok());

    // Check if gateway is running
    assert!(gateway.is_running().await);

    // Check subscriptions
    let subs = gateway.get_subscriptions().await;
    assert!(subs.contains_key("MOCK"));
    assert_eq!(subs.get("MOCK").unwrap().len(), 1);

    // Wait for and check a quote
    let quote = tokio::time::timeout(Duration::from_millis(1000), quote_rx.recv()).await;
    assert!(quote.is_ok());

    let quote = quote.unwrap();
    assert!(quote.is_some());

    let quote = quote.unwrap();
    assert_eq!(quote.symbol, "BTCUSDT");

    // Unsubscribe
    let result = gateway.unsubscribe_all().await;
    assert!(result.is_ok());

    // Check if stopped
    assert!(!gateway.is_running().await);
}

#[tokio::test]
async fn test_quote_gateway_process_quote() {
    // Create channels
    let (quote_tx, mut quote_rx) = mpsc::channel(100);

    // Create gateway
    let gateway = QuoteGateway::new(quote_tx);

    // Process a quote
    let quote = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 50000.0,
        ask: 50001.0,
        bid_size: 1.0,
        ask_size: 1.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    let result = gateway.process_quote(quote.clone()).await;
    assert!(result.is_ok());

    // Check that quote was sent to the receiver
    let received = quote_rx.recv().await;
    assert!(received.is_some());

    let received = received.unwrap();
    assert_eq!(received.symbol, quote.symbol);
    assert_eq!(received.bid, quote.bid);
    assert_eq!(received.ask, quote.ask);
}

#[tokio::test]
async fn test_quote_gateway_multiple_venues() {
    // Create channels
    let (quote_tx, mut quote_rx) = mpsc::channel(100);

    // Create gateway
    let gateway = QuoteGateway::new(quote_tx);

    // Create multiple venues with different configurations
    let mut config1 = MockVenueConfig::default();
    config1.quote_interval_ms = 50; // Faster updates

    let mut config2 = MockVenueConfig::default();
    config2.quote_interval_ms = 100; // Slower updates

    let venue1 = Arc::new(MockVenue::new("MOCK1", config1)
        .with_quote_sender(gateway.quote_tx.clone()));

    let venue2 = Arc::new(MockVenue::new("MOCK2", config2)
        .with_quote_sender(gateway.quote_tx.clone()));

    // Add venues to gateway
    gateway.add_venue(venue1.clone()).await;
    gateway.add_venue(venue2.clone()).await;

    // Subscribe to symbols
    let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    let result = gateway.subscribe(symbols).await;
    assert!(result.is_ok());

    // Collect quotes from both venues for a short period
    let mut quotes = Vec::new();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        if let Ok(Some(quote)) = tokio::time::timeout(Duration::from_millis(200), quote_rx.recv()).await {
            quotes.push(quote);
        }
    }

    // Should have received quotes from both venues
    assert!(!quotes.is_empty());

    // Check that we have quotes from both venues
    let venue1_quotes = quotes.iter().filter(|q| q.venue == "MOCK1").count();
    let venue2_quotes = quotes.iter().filter(|q| q.venue == "MOCK2").count();

    assert!(venue1_quotes > 0);
    assert!(venue2_quotes > 0);

    // Venue1 should have produced more quotes due to faster interval
    assert!(venue1_quotes > venue2_quotes);

    // Unsubscribe
    let result = gateway.unsubscribe_all().await;
    assert!(result.is_ok());
}