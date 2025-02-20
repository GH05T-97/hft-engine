use async_trait::async_trait;
use crate::types::{Order, Quote};
use std::error::Error;
pub use self::adapter::VenueAdapter; 

pub mod binance;
pub use binance::BinanceVenue;

#[async_trait]
pub trait VenueAdapter: Send + Sync {
    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>>;
    async fn submit_order(&self, order: Order) -> Result<String, Box<dyn std::error::Error>>;
}