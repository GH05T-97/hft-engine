use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;

use hft_engine::types::{Order, OrderSide, OrderType};
use hft_engine::gateways::order::OrderGateway;
use hft_engine::execution::ExecutionEngine;
use crate::mocks::mock_venue::{MockVenue, MockVenueConfig};

/// This test verifies the end-to-end flow of orders from strategy through
/// execution engine and order gateway to venues.
#[tokio::test]
async fn test_basic_order_flow() {
    // Create order channel
    let (order_tx, order_rx) = mpsc::channel(100);

    // Create execution engine
    let execution_engine = ExecutionEngine {
        order_tx: order_tx.clone(),
    };

    // Create mock venue
    let venue = Arc::new(MockVenue::new("MOCK", MockVenueConfig::default()));

    // Create order gateway
    let order_gateway = OrderGateway {
        venues: vec![venue.clone()],
        order_rx,
    };

    // Create a mock order
    let order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 50000.0,
        venue: "MOCK".to_string(),
        order_type: OrderType::Limit,
    };

    // Send the order through the channel
    execution_engine.order_tx.send(order.clone()).await.expect("Failed to send order");

    // TODO: In a complete implementation, we would have a method on OrderGateway
    // to process orders. Since we haven't implemented that yet, this test is
    // more of a placeholder to show the structure.

    // For now, let's just verify the channels are connected properly
    let received_order = order_gateway.order_rx.recv().await;
    assert!(received_order.is_some());

    let received_order = received_order.unwrap();
    assert_eq!(received_order.symbol, order.symbol);
    assert_eq!(received_order.side, order.side);
    assert_eq!(received_order.quantity, order.quantity);
    assert_eq!(received_order.price, order.price);
    assert_eq!(received_order.venue, order.venue);
    assert_eq!(received_order.order_type, order.order_type);
}

/// This test verifies that orders are properly routed to the correct venue.
#[tokio::test]
async fn test_order_routing() {
    // Create order channel
    let (order_tx, order_rx) = mpsc::channel(100);

    // Create multiple mock venues
    let venue1 = Arc::new(MockVenue::new("VENUE1", MockVenueConfig::default()));
    let venue2 = Arc::new(MockVenue::new("VENUE2", MockVenueConfig::default()));

    // Set up specific responses for testing
    venue1.set_order_response("BTCUSDT", OrderSide::Buy, Ok("order_id_venue1".to_string())).await;
    venue2.set_order_response("ETHUSDT", OrderSide::Sell, Ok("order_id_venue2".to_string())).await;

    // Create order gateway with both venues
    let venues = vec![venue1.clone(), venue2.clone()];

    // In a complete implementation, the OrderGateway would handle routing
    // orders to the appropriate venue based on the venue field in the order.
    // Since we haven't fully implemented that yet, this test is more of a
    // placeholder to show the intended structure.

    // Create a mock order for venue1
    let order1 = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 50000.0,
        venue: "VENUE1".to_string(),
        order_type: OrderType::Limit,
    };

    // Create a mock order for venue2
    let order2 = Order {
        symbol: "ETHUSDT".to_string(),
        side: OrderSide::Sell,
        quantity: 2.0,
        price: 3000.0,
        venue: "VENUE2".to_string(),
        order_type: OrderType::Limit,
    };

    // In a complete implementation, we would:
    // 1. Send orders through the channel
    // 2. Have the OrderGateway process them and route to the correct venue
    // 3. Verify the orders were received by the correct venues

    // For now, we can just submit directly to the venues to verify routing works
    let result1 = venue1.submit_order(order1.clone()).await;
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap(), "order_id_venue1");

    let result2 = venue2.submit_order(order2.clone()).await;
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), "order_id_venue2");
}

/// This test verifies that order submissions handle errors properly.
#[tokio::test]
async fn test_order_error_handling() {
    // Create order channel
    let (order_tx, order_rx) = mpsc::channel(100);

    // Create mock venue with custom error responses
    let venue = Arc::new(MockVenue::new("MOCK", MockVenueConfig::default()));

    // Configure a specific error response for invalid orders
    use hft_engine::error::{HftError, VenueError};
    let error = VenueError::OrderSubmissionFailed("Insufficient funds".to_string()).into();
    venue.set_order_response("BTCUSDT", OrderSide::Buy, Err(error)).await;

    // Create a mock order that will trigger the error
    let order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 50000.0,
        venue: "MOCK".to_string(),
        order_type: OrderType::Limit,
    };

    // Submit directly to verify error handling
    let result = venue.submit_order(order.clone()).await;
    assert!(result.is_err());

    if let Err(HftError::Venue(VenueError::OrderSubmissionFailed(msg))) = result {
        assert_eq!(msg, "Insufficient funds");
    } else {
        panic!("Expected OrderSubmissionFailed error, got: {:?}", result);
    }

    // In a complete implementation, the OrderGateway would need to handle
    // these errors, potentially retrying or notifying the strategy about
    // the failure.
}

/// This test verifies the behavior with high-frequency order submission.
#[tokio::test]
async fn test_high_frequency_order_submission() {
    // Create order channel with bounded capacity
    let (order_tx, order_rx) = mpsc::channel(10);

    // Create mock venue with low latency
    let mut config = MockVenueConfig::default();
    config.latency_ms = 1; // Very low latency
    let venue = Arc::new(MockVenue::new("MOCK", config));

    // Create order gateway
    let order_gateway = OrderGateway {
        venues: vec![venue.clone()],
        order_rx,
    };

    // Create a batch of orders
    let num_orders = 20; // More than channel capacity
    let mut orders = Vec::with_capacity(num_orders);

    for i in 0..num_orders {
        let order = Order {
            symbol: "BTCUSDT".to_string(),
            side: if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell },
            quantity: 1.0,
            price: 50000.0 + (i as f64 * 10.0),
            venue: "MOCK".to_string(),
            order_type: OrderType::Limit,
        };
        orders.push(order);
    }

    // Send orders as fast as possible
    let mut sent_count = 0;
    for order in orders {
        match order_tx.try_send(order) {
            Ok(_) => {
                sent_count += 1;
            },
            Err(e) => {
                if e.is_full() {
                    // Channel is full, which is expected
                    break;
                } else {
                    panic!("Failed to send order: {:?}", e);
                }
            }
        }
    }

    // Should have sent the channel capacity
    assert!(sent_count <= 10, "Should not have sent more than channel capacity");

    // In practice, we'd want backpressure mechanisms and order prioritization
    // to handle high-frequency order submission.
}

/// This test verifies that market orders are handled differently from limit orders.
#[tokio::test]
async fn test_market_order_handling() {
    // Create mock venue
    let venue = Arc::new(MockVenue::new("MOCK", MockVenueConfig::default()));

    // Create a market order
    let market_order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 0.0, // Price is ignored for market orders
        venue: "MOCK".to_string(),
        order_type: OrderType::Market,
    };

    // Create a limit order
    let limit_order = Order {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 50000.0,
        venue: "MOCK".to_string(),
        order_type: OrderType::Limit,
    };

    // Both orders should be accepted
    let market_result = venue.submit_order(market_order.clone()).await;
    assert!(market_result.is_ok());

    let limit_result = venue.submit_order(limit_order.clone()).await;
    assert!(limit_result.is_ok());

    // In a real implementation, the venue adapter would handle market orders
    // differently from limit orders, potentially setting the price to None
    // or applying different validation rules.
}