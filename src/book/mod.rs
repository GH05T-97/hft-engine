use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use crate::types::Quote;
use crate::metrics::ORDERBOOK_UPDATES;

pub struct BookBuilder {
    pub(crate) books: Arc<RwLock<HashMap<String, OrderBook>>>,
    pub(crate) quote_rx: mpsc::Receiver<Quote>,
}

impl BookBuilder {
    async fn process_quote(&self, quote: Quote) {
        // Process quote logic here

        ORDERBOOK_UPDATES
            .with_label_values(&[&quote.symbol])
            .inc();
    }
}

pub struct OrderBook {
    symbol: String,
    bids: BTreeMap<f64, f64>,
    asks: BTreeMap<f64, f64>,
}
