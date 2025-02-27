use tokio::sync::mpsc;
use std::time::Duration;

use hft_engine::venues::binance::BinanceVenue;
use hft_engine::venues::VenueAdapter;
use hft_engine::types::{Order, OrderSide, OrderType, Quote};
use hft_engine::error::{HftError, VenueError};

#[tokio::test]
async fn test_binance_venue_name() {
    let venue = BinanceVenue::new(
        "fake_api_key".to_string(),
        "fake_api_secret".to_string(),
    );

    assert_eq!(venue.name().await, "BINANCE_FUTURES");
}

#[tokio::test]
async fn test_binance_invalid_order_quantity() {
    let venue = BinanceVenue::new(
        "fake_api_key".to_string(),
        "fake_api_secret".to_string(),
    );

    let order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: -1.0, // Invalid quantity
        price: 50000.0,
        venue: "BINANCE".to_string(),
        order_type: OrderType::Limit,
    };

    let result = venue.submit_order(order).await;
    assert!(result.is_err());

    if let Err(HftError::Venue(VenueError::OrderSubmissionFailed(msg))) = result {
        assert!(msg.contains("Invalid quantity"));
    } else {
        panic!("Expected OrderSubmissionFailed error, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_binance_invalid_limit_price() {
    let venue = BinanceVenue::new(
        "fake_api_key".to_string(),
        "fake_api_secret".to_string(),
    );

    let order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 0.0, // Invalid price for limit order
        venue: "BINANCE".to_string(),
        order_type: OrderType::Limit,
    };

    let result = venue.submit_order(order).await;
    assert!(result.is_err());

    if let Err(HftError::Venue(VenueError::OrderSubmissionFailed(msg))) = result {
        assert!(msg.contains("Invalid price for limit order"));
    } else {
        panic!("Expected OrderSubmissionFailed error, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_market_order_zero_price() {
    // Market orders can have a zero price
    let venue = BinanceVenue::new(
        "fake_api_key".to_string(),
        "fake_api_secret".to_string(),
    );

    let order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 0.0, // Valid for market orders
        venue: "BINANCE".to_string(),
        order_type: OrderType::Market,
    };

    let result = venue.submit_order(order).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_venue_with_quote_sender() {
    let (tx, mut rx) = mpsc::channel::<Quote>(100);

    let venue = BinanceVenue::new(
        "fake_api_key".to_string(),
        "fake_api_secret".to_string(),
    ).with_quote_sender(tx);

    // Since we can't easily test the websocket connection without mocking external services,
    // we'll just test that the venue is properly configured with the quote sender.
    // The actual connection would be tested in an integration test with proper mocking.

    assert_eq!(venue.name().await, "BINANCE_FUTURES");

    // Testing that submit_order still works with the quote sender configured
    let order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 50000.0,
        venue: "BINANCE".to_string(),
        order_type: OrderType::Limit,
    };

    let result = venue.submit_order(order).await;
    assert!(result.is_ok());
}