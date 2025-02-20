use std::sync::Arc;
use tokio::sync::mpsc;
use crate::types::Quote;
use crate::venues::VenueAdapter;

pub struct QuoteGateway {
    pub(crate) venues: Vec<Arc<dyn VenueAdapter>>,
    pub(crate) quote_tx: mpsc::Sender<Quote>,
}