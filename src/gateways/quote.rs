use std::sync::Arc;
use tokio::sync::mpsc;
use crate::types::Quote;
use crate::venues::VenueAdapter;

pub struct QuoteGateway {
    pub(crate) venues: Vec<Arc<dyn VenueAdapter>>,
    pub(crate) quote_tx: mpsc::Sender<Quote>,
}

impl QuoteGateway {
    pub fn add_venue(&mut self, venue: Arc<dyn VenueAdapter>) {
        self.venues.push(venue);
    }

    pub async fn subscribe(&self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        for venue in &self.venues {
            venue.subscribe_quotes(symbols.clone()).await?;
        }
        Ok(())
    }
}