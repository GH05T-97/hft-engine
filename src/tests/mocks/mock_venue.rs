use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, sleep};
use async_trait::async_trait;
use rand::Rng;

use crate::error::{HftError, VenueError};
use crate::types::{Order, Quote, OrderSide, OrderType};
use crate::venues::VenueAdapter;

pub struct MockVenueConfig {
    pub symbol_base_prices: HashMap<String, f64>,
    pub quote_interval_ms: u64,
    pub latency_ms: u64,
    pub error_probability: f64,
    pub disconnect_probability: f64,
}

impl Default for MockVenueConfig {
    fn default() -> Self {
        let mut symbol_base_prices = HashMap::new();
        symbol_base_prices.insert("BTCUSDT".to_string(), 50000.0);
        symbol_base_prices.insert("ETHUSDT".to_string(), 3000.0);

        Self {
            symbol_base_prices,
            quote_interval_ms: 100,
            latency_ms: 5,
            error_probability: 0.01,
            disconnect_probability: 0.001,
        }
    }
}

pub struct MockVenue {
    name: String,
    config: MockVenueConfig,
    subscribed_symbols: Arc<RwLock<Vec<String>>>,
    quote_tx: Option<mpsc::Sender<Quote>>,
    is_running: Arc<RwLock<bool>>,
    order_responses: Arc<RwLock<HashMap<String, Result<String, HftError>>>>,
}

impl MockVenue {
    pub fn new(name: &str, config: MockVenueConfig) -> Self {
        Self {
            name: name.to_string(),
            config,
            subscribed_symbols: Arc::new(RwLock::new(Vec::new())),
            quote_tx: None,
            is_running: Arc::new(RwLock::new(false)),
            order_responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_quote_sender(mut self, quote_tx: mpsc::Sender<Quote>) -> Self {
        self.quote_tx = Some(quote_tx);
        self
    }

    // Configure a specific response for an order with the given symbol and side
    pub async fn set_order_response(&self, symbol: &str, side: OrderSide, response: Result<String, HftError>) {
        let key = format!("{}:{:?}", symbol, side);
        let mut responses = self.order_responses.write().await;
        responses.insert(key, response);
    }

    // Start generating mock quotes
    async fn start_quote_generation(&self) -> Result<(), HftError> {
        if self.quote_tx.is_none() {
            return Err(VenueError::ConnectionFailed("Quote sender not configured".to_string()).into());
        }

        let quote_tx = self.quote_tx.as_ref().unwrap().clone();
        let subscribed_symbols = self.subscribed_symbols.clone();
        let config = self.config.clone();
        let venue_name = self.name.clone();
        let is_running = self.is_running.clone();

        *is_running.write().await = true;

        tokio::spawn(async move {
            let mut rng = rand::thread_rng();

            while *is_running.read().await {
                let symbols = subscribed_symbols.read().await.clone();

                for symbol in &symbols {
                    // Simulate random connectivity issues
                    if rng.gen::<f64>() < config.disconnect_probability {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    // Simulate random errors
                    if rng.gen::<f64>() < config.error_probability {
                        continue;
                    }

                    // Get base price for this symbol
                    let base_price = *config.symbol_base_prices.get(symbol).unwrap_or(&100.0);

                    // Generate random price movements (±0.5%)
                    let price_movement = (rng.gen::<f64>() - 0.5) * 0.01 * base_price;
                    let mid_price = base_price + price_movement;

                    // Create spread around mid price
                    let spread = mid_price * 0.0002; // 0.02% spread
                    let bid = mid_price - spread / 2.0;
                    let ask = mid_price + spread / 2.0;

                    // Random sizes
                    let bid_size = rng.gen_range(0.1..10.0);
                    let ask_size = rng.gen_range(0.1..10.0);

                    let quote = Quote {
                        symbol: symbol.clone(),
                        bid,
                        ask,
                        bid_size,
                        ask_size,
                        venue: venue_name.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    };

                    // Simulate network latency
                    sleep(Duration::from_millis(config.latency_ms)).await;

                    // Send quote
                    if let Err(e) = quote_tx.send(quote).await {
                        eprintln!("Failed to send mock quote: {}", e);
                        break;
                    }
                }

                // Wait before next update
                sleep(Duration::from_millis(config.quote_interval_ms)).await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        *self.is_running.write().await = false;
    }
}

impl Clone for MockVenueConfig {
    fn clone(&self) -> Self {
        Self {
            symbol_base_prices: self.symbol_base_prices.clone(),
            quote_interval_ms: self.quote_interval_ms,
            latency_ms: self.latency_ms,
            error_probability: self.error_probability,
            disconnect_probability: self.disconnect_probability,
        }
    }
}

#[async_trait]
impl VenueAdapter for MockVenue {
    async fn name(&self) -> String {
        self.name.clone()
    }

    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), HftError> {
        if symbols.is_empty() {
            return Err(VenueError::SubscriptionFailed("Empty symbol list".to_string()).into());
        }

        // Store subscribed symbols
        {
            let mut subscribed = self.subscribed_symbols.write().await;
            subscribed.clear();
            subscribed.extend(symbols);
        }

        // Start generating quotes if not already running
        if !*self.is_running.read().await {
            self.start_quote_generation().await?;
        }

        Ok(())
    }

    async fn submit_order(&self, order: Order) -> Result<String, HftError> {
        // Simulate network latency
        sleep(Duration::from_millis(self.config.latency_ms)).await;

        // Check for configured response
        let key = format!("{}:{:?}", order.symbol, order.side);
        let responses = self.order_responses.read().await;

        if let Some(response) = responses.get(&key) {
            return response.clone();
        }

        // Default behavior
        let mut rng = rand::thread_rng();

        // Validate order parameters
        if order.quantity <= 0.0 {
            return Err(VenueError::OrderSubmissionFailed(
                format!("Invalid quantity: {}", order.quantity)
            ).into());
        }

        if order.price <= 0.0 && matches!(order.order_type, OrderType::Limit) {
            return Err(VenueError::OrderSubmissionFailed(
                format!("Invalid price for limit order: {}", order.price)
            ).into());
        }

        // Random failure simulation
        if rng.gen::<f64>() < self.config.error_probability {
            return Err(VenueError::OrderSubmissionFailed("Random failure".to_string()).into());
        }

        // Generate mock order ID
        let order_id = format!("mock_order_{}_{}",
            order.symbol.to_lowercase(),
            chrono::Utc::now().timestamp_millis()
        );

        Ok(order_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_venue_subscribe() {
        let (tx, mut rx) = mpsc::channel(100);

        let venue = MockVenue::new("MOCK", MockVenueConfig::default())
            .with_quote_sender(tx);

        let symbols = vec!["BTCUSDT".to_string()];
        let result = venue.subscribe_quotes(symbols).await;
        assert!(result.is_ok());

        // Check that we receive some quotes
        let quote = tokio::time::timeout(Duration::from_millis(1000), rx.recv()).await;
        assert!(quote.is_ok());
        let quote = quote.unwrap();
        assert!(quote.is_some());
        let quote = quote.unwrap();
        assert_eq!(quote.symbol, "BTCUSDT");

        venue.stop().await;
    }

    #[tokio::test]
    async fn test_mock_venue_order_response() {
        let venue = MockVenue::new("MOCK", MockVenueConfig::default());

        // Configure a specific error response
        let error = VenueError::OrderSubmissionFailed("Insufficient funds".to_string()).into();
        venue.set_order_response("BTCUSDT", OrderSide::Buy, Err(error)).await;

        let order = Order {
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            quantity: 1.0,
            price: 50000.0,
            venue: "MOCK".to_string(),
            order_type: OrderType::Limit,
        };

        let result = venue.submit_order(order).await;
        assert!(result.is_err());

        // Configure a specific success response
        venue.set_order_response("ETHUSDT", OrderSide::Sell, Ok("specific_order_id".to_string())).await;

        let order = Order {
            symbol: "ETHUSDT".to_string(),
            side: OrderSide::Sell,
            quantity: 1.0,
            price: 3000.0,
            venue: "MOCK".to_string(),
            order_type: OrderType::Limit,
        };

        let result = venue.submit_order(order).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "specific_order_id");
    }
}