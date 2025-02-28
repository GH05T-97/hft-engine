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
        let mut books = self.books.write().await;

        let book = books
            .entry(quote.symbol.clone())
            .or_insert_with(|| OrderBook::new(quote.symbol.clone()));

        book.update(&quote);

        ORDERBOOK_UPDATES
            .with_label_values(&[&quote.symbol])
            .inc();
    }

    pub async fn run(&mut self) {
        while let Some(quote) = self.quote_rx.recv().await {
            self.process_quote(quote).await;
        }
    }
}

pub struct OrderBook {
    symbol: String,
    bids: BTreeMap<i64, f64>,
    asks: BTreeMap<i64, f64>,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn update(&mut self, quote: &Quote) {
        const PRICE_MULTIPLIER: f64 = 100_000_000.0;

        if quote.bid > 0.0 {
            let bid_price = (quote.bid * PRICE_MULTIPLIER) as i64;
            self.bids.insert(bid_price, quote.bid_size);
        }
        if quote.ask > 0.0 {
            let ask_price = (quote.ask * PRICE_MULTIPLIER) as i64;
            self.asks.insert(ask_price, quote.ask_size);
        }
    }

    pub fn best_bid(&self) -> Option<(f64, f64)> {
        const PRICE_MULTIPLIER: f64 = 100_000_000.0;
        self.bids.iter().next_back()
            .map(|(&p, &s)| ((p as f64) / PRICE_MULTIPLIER, s))
    }

    pub fn best_ask(&self) -> Option<(f64, f64)> {
        const PRICE_MULTIPLIER: f64 = 100_000_000.0;
        self.asks.iter().next()
            .map(|(&p, &s)| ((p as f64) / PRICE_MULTIPLIER, s))
    }
}

