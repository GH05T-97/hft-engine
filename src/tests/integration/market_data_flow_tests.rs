use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;

use hft_engine::book::{OrderBook, BookBuilder};
use hft_engine::types::Quote;
use hft_engine::gateways::quote::QuoteGateway;
use crate::mocks::mock_venue::{MockVenue, MockVenueConfig};

/// This test verifies the end-to-end flow of market data from venues through
/// the quote gateway to the order book builder.
#[tokio::test]
async fn test_market_data_flow() {
    // Create channels for quote flow
    let (quote_tx, quote_rx) = mpsc::channel(1000);

    // Create shared order books collection
    let books = Arc::new(RwLock::new(HashMap::new()));

    // Create components
    let gateway = QuoteGateway::new(quote_tx);
    let mut book_builder = BookBuilder {
        books: Arc::clone(&books),
        quote_rx,
    };

    // Create and add a mock venue
    let venue = Arc::new(MockVenue::new("MOCK", MockVenueConfig::default())
        .with_quote_sender(gateway.quote_tx.clone()));

    gateway.add_venue(venue.clone()).await;

    // Start the book builder in a separate task
    let book_builder_handle = tokio::spawn(async move {
        book_builder.run().await;
    });

    // Subscribe to a symbol
    let symbols = vec!["BTCUSDT".to_string()];
    gateway.subscribe(symbols).await.expect("Failed to subscribe to symbols");

    // Give some time for quotes to flow through the system
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that the order book was created and populated
    let books_read = books.read().await;
    assert!(books_read.contains_key("BTCUSDT"));

    let book = books_read.get("BTCUSDT").unwrap();

    // Book should have bids and asks
    assert!(!book.bids.is_empty());
    assert!(!book.asks.is_empty());

    // Best bid and ask should exist and make sense (bid < ask)
    let best_bid = book.best_bid().unwrap();
    let best_ask = book.best_ask().unwrap();

    assert!(best_bid.0 < best_ask.0, "Best bid ({}) should be less than best ask ({})", best_bid.0, best_ask.0);

    // Clean up
    gateway.unsubscribe_all().await.expect("Failed to unsubscribe");

    // Normally we would cancel the book_builder_handle here, but since we can't easily
    // signal it to stop, we'll just let it be dropped at the end of the test.
}

/// This test verifies that the system can handle multiple symbols
/// across multiple venues.
#[tokio::test]
async fn test_multi_symbol_multi_venue_flow() {
    // Create channels for quote flow
    let (quote_tx, quote_rx) = mpsc::channel(1000);

    // Create shared order books collection
    let books = Arc::new(RwLock::new(HashMap::new()));

    // Create components
    let gateway = QuoteGateway::new(quote_tx);
    let mut book_builder = BookBuilder {
        books: Arc::clone(&books),
        quote_rx,
    };

    // Create and add multiple mock venues
    let venue1 = Arc::new(MockVenue::new("VENUE1", MockVenueConfig::default())
        .with_quote_sender(gateway.quote_tx.clone()));

    let venue2 = Arc::new(MockVenue::new("VENUE2", MockVenueConfig::default())
        .with_quote_sender(gateway.quote_tx.clone()));

    gateway.add_venue(venue1.clone()).await;
    gateway.add_venue(venue2.clone()).await;

    // Start the book builder in a separate task
    let book_builder_handle = tokio::spawn(async move {
        book_builder.run().await;
    });

    // Subscribe to multiple symbols
    let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    gateway.subscribe(symbols).await.expect("Failed to subscribe to symbols");

    // Give some time for quotes to flow through the system
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that order books were created and populated for both symbols
    let books_read = books.read().await;
    assert!(books_read.contains_key("BTCUSDT"));
    assert!(books_read.contains_key("ETHUSDT"));

    // Both books should have bids and asks
    for symbol in &["BTCUSDT", "ETHUSDT"] {
        let book = books_read.get(*symbol).unwrap();

        assert!(!book.bids.is_empty(), "Book for {} should have bids", symbol);
        assert!(!book.asks.is_empty(), "Book for {} should have asks", symbol);

        // Best bid and ask should exist and make sense (bid < ask)
        let best_bid = book.best_bid().unwrap();
        let best_ask = book.best_ask().unwrap();

        assert!(best_bid.0 < best_ask.0,
            "For {}: Best bid ({}) should be less than best ask ({})",
            symbol, best_bid.0, best_ask.0);
    }

    // Clean up
    gateway.unsubscribe_all().await.expect("Failed to unsubscribe");
}

/// This test verifies the system's resilience when a venue disconnects
/// but another venue is still available.
#[tokio::test]
async fn test_venue_disconnect_resilience() {
    // Create channels for quote flow
    let (quote_tx, quote_rx) = mpsc::channel(1000);

    // Create shared order books collection
    let books = Arc::new(RwLock::new(HashMap::new()));

    // Create components
    let gateway = QuoteGateway::new(quote_tx);
    let mut book_builder = BookBuilder {
        books: Arc::clone(&books),
        quote_rx,
    };

    // Create mock venues with one having higher error rate
    let mut reliable_config = MockVenueConfig::default();
    reliable_config.error_probability = 0.0;
    reliable_config.disconnect_probability = 0.0;

    let mut unreliable_config = MockVenueConfig::default();
    unreliable_config.error_probability = 0.5; // 50% error rate
    unreliable_config.disconnect_probability = 0.5; // 50% disconnect rate

    let reliable_venue = Arc::new(MockVenue::new("RELIABLE", reliable_config)
        .with_quote_sender(gateway.quote_tx.clone()));

    let unreliable_venue = Arc::new(MockVenue::new("UNRELIABLE", unreliable_config)
        .with_quote_sender(gateway.quote_tx.clone()));

    gateway.add_venue(reliable_venue.clone()).await;
    gateway.add_venue(unreliable_venue.clone()).await;

    // Start the book builder in a separate task
    let book_builder_handle = tokio::spawn(async move {
        book_builder.run().await;
    });

    // Subscribe to a symbol
    let symbols = vec!["BTCUSDT".to_string()];
    gateway.subscribe(symbols).await.expect("Failed to subscribe to symbols");

    // Give some time for quotes to flow through the system
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Check that the order book was created and populated
    let books_read = books.read().await;
    assert!(books_read.contains_key("BTCUSDT"));

    let book = books_read.get("BTCUSDT").unwrap();

    // Book should have bids and asks from the reliable venue
    assert!(!book.bids.is_empty());
    assert!(!book.asks.is_empty());

    // Even with an unreliable venue, the system should still work
    let best_bid = book.best_bid().unwrap();
    let best_ask = book.best_ask().unwrap();

    assert!(best_bid.0 < best_ask.0, "Best bid ({}) should be less than best ask ({})", best_bid.0, best_ask.0);

    // Clean up
    gateway.unsubscribe_all().await.expect("Failed to unsubscribe");
}