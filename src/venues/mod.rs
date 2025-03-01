use async_trait::async_trait;
use crate::types::{Order, Quote};
use crate::error::HftError;

pub mod binance;
pub use binance::BinanceVenue;

#[async_trait]
pub trait VenueAdapter: Send + Sync {
    /// Get the venue name
    async fn name(&self) -> String;

    /// Subscribe to quotes for the given symbols
    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), HftError>;

    /// Submit an order to the venue
    async fn submit_order(&self, order: Order) -> Result<String, HftError>;
    
    /// Stop any background tasks or connections
    async fn stop(&self) -> Result<(), HftError> {
        // Default implementation does nothing
        Ok(())
    }
}