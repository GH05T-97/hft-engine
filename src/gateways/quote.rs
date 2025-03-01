use std::sync::Arc;
use std::any::Any;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tracing::{info, warn, error, debug};

use crate::types::Quote;
use crate::venues::VenueAdapter;
use crate::error::{HftError, GatewayError};
use crate::metrics::QUOTE_GATEWAY_THROUGHPUT;

#[cfg(test)]
use crate::mocks::mock_venue::{MockVenue, MockVenueConfig};
pub struct QuoteGateway {
    pub(crate) venues: RwLock<Vec<Arc<dyn VenueAdapter>>>,
    pub(crate) quote_tx: mpsc::Sender<Quote>,
    pub(crate) subscriptions: RwLock<HashMap<String, Vec<String>>>,
    pub(crate) is_running: RwLock<bool>,
}

impl QuoteGateway {
    pub fn new(quote_tx: mpsc::Sender<Quote>) -> Self {
        Self {
            venues: RwLock::new(Vec::new()),
            quote_tx,
            subscriptions: RwLock::new(HashMap::new()),
            is_running: RwLock::new(false),
        }
    }

    /// Add a venue to the quote gateway
    pub async fn add_venue(&self, venue: Arc<dyn VenueAdapter>) {
        let venue_name = venue.name().await;
        debug!(venue = %venue_name, "Adding venue to quote gateway");

        let mut venues = self.venues.write().await;
        venues.push(venue.clone());

        // If we already have subscriptions and the gateway is running,
        // subscribe the new venue to existing symbols
        if *self.is_running.read().await {
            let subscriptions = self.subscriptions.read().await;
            for (venue_name, symbols) in subscriptions.iter() {
                if venue_name == &venue.name().await && !symbols.is_empty() {
                    if let Err(e) = venue.subscribe_quotes(symbols.clone()).await {
                        error!(
                            venue = %venue_name,
                            symbols = ?symbols,
                            error = ?e,
                            "Failed to subscribe new venue to existing symbols"
                        );
                    }
                }
            }
        }
    }

    pub async fn remove_venue(&self, venue_name: &str) -> Result<(), HftError> {
        debug!(venue = %venue_name, "Removing venue from quote gateway");

        let mut venues = self.venues.write().await;
        let original_len = venues.len();

        // Create a new vector to hold venues we want to keep
        let mut new_venues = Vec::new();
        let mut removed_venue = None;

        // Check each venue and only keep those with a different name
        for venue in venues.drain(..) {
            if venue.name().await != venue_name {
                new_venues.push(venue);
            } else {
                removed_venue = Some(venue);
            }
        }

        // Check if we actually removed any venue
        if new_venues.len() == original_len {
            warn!(venue = %venue_name, "Attempted to remove venue that was not found");

            // Put all venues back since we didn't find the one to remove
            *venues = new_venues;
            return Err(GatewayError::VenueNotFound(venue_name.to_string()).into());
        }

        // Update the venues with our filtered list
        *venues = new_venues;

        // Stop the removed venue if we found one
        if let Some(venue) = removed_venue {
            venue.stop().await?;
        }

        Ok(())
    }

    /// Subscribe to quotes for the given symbols on all venues
    pub async fn subscribe(&self, symbols: Vec<String>) -> Result<(), HftError> {
        if symbols.is_empty() {
            return Err(GatewayError::InvalidSymbol("Empty symbol list".to_string()).into());
        }

        info!(symbols = ?symbols, "Subscribing to symbols");

        let venues = self.venues.read().await;
        if venues.is_empty() {
            return Err(GatewayError::NoVenuesConfigured.into());
        }

        // Track subscription errors
        let mut errors = Vec::new();

        // Subscribe each venue to the symbols
        for venue in venues.iter() {
            let venue_name = venue.name().await;
            debug!(venue = %venue_name, symbols = ?symbols, "Subscribing venue to symbols");

            match venue.subscribe_quotes(symbols.clone()).await {
                Ok(_) => {
                    debug!(venue = %venue_name, "Subscription successful");
                    // Store successful subscription
                    let mut subscriptions = self.subscriptions.write().await;
                    subscriptions.insert(venue_name, symbols.clone());
                },
                Err(e) => {
                    error!(venue = %venue_name, error = ?e, "Failed to subscribe to symbols");
                    errors.push((venue_name, e));
                }
            }
        }

        // If all venues failed, return an error
        if errors.len() == venues.len() {
            let error_msg = errors.into_iter()
                .map(|(venue, err)| format!("{}: {:?}", venue, err))
                .collect::<Vec<_>>()
                .join(", ");

            return Err(GatewayError::SubscriptionFailed(error_msg).into());
        }

        *self.is_running.write().await = true;

        Ok(())
    }

    /// Process an incoming quote from a venue
    pub async fn process_quote(&self, quote: Quote) -> Result<(), HftError> {
        // Update metrics
        let symbol = quote.symbol.clone();
        QUOTE_GATEWAY_THROUGHPUT
            .with_label_values(&[&symbol, &quote.venue])
            .inc();

        // Forward the quote to the book builder
        self.quote_tx.send(quote).await
            .map_err(|e| GatewayError::ChannelSendFailed(format!("Failed to send quote: {}", e)))?;

        Ok(())
    }

    /// Unsubscribe from all symbols
    pub async fn unsubscribe_all(&self) -> Result<(), HftError> {
        info!("Unsubscribing from all symbols");

        // Set running state to false
        *self.is_running.write().await = false;

        // Clear subscriptions
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.clear();

        Ok(())
    }

    /// Check if the gateway is currently running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get the current subscriptions
    pub async fn get_subscriptions(&self) -> HashMap<String, Vec<String>> {
        self.subscriptions.read().await.clone()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::time::Duration;
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
    drop(venues);  // Explicitly drop the lock

    // Stop venues explicitly for test cleanup
    venue1.stop().await;
    venue2.stop().await;

    // Remove one venue
    let result = gateway.remove_venue("MOCK1").await;
    assert!(result.is_ok());

    // Check that venue was removed
    let venues = gateway.venues.read().await;
    assert_eq!(venues.len(), 1);
    drop(venues);  // Explicitly drop the lock

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
}
