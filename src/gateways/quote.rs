use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error, debug};
use async_trait::async_trait;

use crate::types::Quote;
use crate::venues::VenueAdapter;
use crate::error::{HftError, GatewayError};
use crate::metrics::QUOTE_GATEWAY_THROUGHPUT;

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
                            error = %e,
                            "Failed to subscribe new venue to existing symbols"
                        );
                    }
                }
            }
        }
    }

    /// Remove a venue from the quote gateway
    pub async fn remove_venue(&self, venue_name: &str) -> Result<(), HftError> {
        debug!(venue = %venue_name, "Removing venue from quote gateway");

        let mut venues = self.venues.write().await;
        let original_len = venues.len();

        // Create a new vector to hold venues we want to keep
        let mut new_venues = Vec::new();

        // Check each venue and only keep those with a different name
        for venue in venues.drain(..) {
            if venue.name().await != venue_name {
                new_venues.push(venue);
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
                    error!(venue = %venue_name, error = %e, "Failed to subscribe to symbols");
                    errors.push((venue_name, e));
                }
            }
        }

        // If all venues failed, return an error
        if errors.len() == venues.len() {
            let error_msg = errors.into_iter()
                .map(|(venue, err)| format!("{}: {}", venue, err))
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

// Add this trait to the VenueAdapter to get the venue name
#[async_trait]
pub trait VenueInfo {
    /// Get the venue name
    async fn name(&self) -> String;
}