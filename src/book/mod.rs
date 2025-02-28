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

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tokio::task;
    use std::collections::HashMap;
    use crate::types::Quote;

    #[tokio::test]
    async fn test_order_book_creation() {
        let book = OrderBook::new("BTCUSDT".to_string());
        assert_eq!(book.symbol, "BTCUSDT");
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[tokio::test]
async fn test_order_book_creation() {
    let book = OrderBook::new("BTCUSDT".to_string());
    assert_eq!(book.symbol, "BTCUSDT");
    assert!(book.bids.is_empty());
    assert!(book.asks.is_empty());
}

#[tokio::test]
async fn test_order_book_update() {
    let mut book = OrderBook::new("BTCUSDT".to_string());

    let quote = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 50000.0,
        ask: 50001.0,
        bid_size: 1.5,
        ask_size: 2.5,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote);

    // Check that the quote was processed correctly
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.len(), 1);

    let (bid_price, bid_size) = book.best_bid().unwrap();
    let (ask_price, ask_size) = book.best_ask().unwrap();

    assert_eq!(bid_price, 50000.0);
    assert_eq!(bid_size, 1.5);
    assert_eq!(ask_price, 50001.0);
    assert_eq!(ask_size, 2.5);
}

#[tokio::test]
async fn test_price_normalization() {
    const PRICE_MULTIPLIER: f64 = 100_000_000.0;

    let price = 50000.12345678;
    let normalized = (price * PRICE_MULTIPLIER) as i64;
    let denormalized = (normalized as f64) / PRICE_MULTIPLIER;

    // Check that price is correctly normalized and denormalized
    assert_eq!(denormalized, price);

    // Test in the context of order book
    let mut book = OrderBook::new("BTCUSDT".to_string());

    let quote = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: price,
        ask: price + 0.00000001, // Test smallest price increment
        bid_size: 1.0,
        ask_size: 1.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote);

    let (bid_price, _) = book.best_bid().unwrap();
    let (ask_price, _) = book.best_ask().unwrap();

    // Verify that the precise price was maintained
    assert_eq!(bid_price, price);
    assert_eq!(ask_price, price + 0.00000001);
}

#[tokio::test]
async fn test_order_book_multiple_levels() {
    let mut book = OrderBook::new("BTCUSDT".to_string());

    // Add multiple price levels
    let quotes = [
        Quote {
            symbol: "BTCUSDT".to_string(),
            bid: 50000.0,
            ask: 50010.0,
            bid_size: 1.0,
            ask_size: 1.0,
            venue: "TEST".to_string(),
            timestamp: 0,
        },
        Quote {
            symbol: "BTCUSDT".to_string(),
            bid: 49990.0,
            ask: 50020.0,
            bid_size: 2.0,
            ask_size: 2.0,
            venue: "TEST".to_string(),
            timestamp: 0,
        },
        Quote {
            symbol: "BTCUSDT".to_string(),
            bid: 49980.0,
            ask: 50030.0,
            bid_size: 3.0,
            ask_size: 3.0,
            venue: "TEST".to_string(),
            timestamp: 0,
        },
    ];

    for quote in &quotes {
        book.update(quote);
    }

    // Check book state
    assert_eq!(book.bids.len(), 3);
    assert_eq!(book.asks.len(), 3);

    // Best bid should be the highest
    let (bid_price, bid_size) = book.best_bid().unwrap();
    assert_eq!(bid_price, 50000.0);
    assert_eq!(bid_size, 1.0);

    // Best ask should be the lowest
    let (ask_price, ask_size) = book.best_ask().unwrap();
    assert_eq!(ask_price, 50010.0);
    assert_eq!(ask_size, 1.0);
}

#[tokio::test]
async fn test_order_book_update_existing_level() {
    let mut book = OrderBook::new("BTCUSDT".to_string());

    // Add initial quote
    let quote1 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 50000.0,
        ask: 50010.0,
        bid_size: 1.0,
        ask_size: 1.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote1);

    // Update with new sizes at same prices
    let quote2 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 50000.0,
        ask: 50010.0,
        bid_size: 2.0,
        ask_size: 3.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote2);

    // Check sizes were updated
    let (_, bid_size) = book.best_bid().unwrap();
    let (_, ask_size) = book.best_ask().unwrap();

    assert_eq!(bid_size, 2.0);
    assert_eq!(ask_size, 3.0);

    // Number of levels should still be 1
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.len(), 1);
}

#[tokio::test]
async fn test_order_book_remove_level() {
    let mut book = OrderBook::new("BTCUSDT".to_string());

    // Add initial quote
    let quote1 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 50000.0,
        ask: 50010.0,
        bid_size: 1.0,
        ask_size: 1.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote1);

    // Add a second level
    let quote2 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 49990.0,
        ask: 50020.0,
        bid_size: 2.0,
        ask_size: 2.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote2);
    assert_eq!(book.bids.len(), 2);
    assert_eq!(book.asks.len(), 2);

    // Remove the top bid and bottom ask by setting size to 0
    let quote3 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 50000.0,
        ask: 50010.0,
        bid_size: 0.0, // This should remove the level
        ask_size: 0.0, // This should remove the level
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote3);

    // Check levels were removed
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.len(), 1);

    // Check best levels are now the second ones
    let (bid_price, bid_size) = book.best_bid().unwrap();
    let (ask_price, ask_size) = book.best_ask().unwrap();

    assert_eq!(bid_price, 49990.0);
    assert_eq!(bid_size, 2.0);
    assert_eq!(ask_price, 50020.0);
    assert_eq!(ask_size, 2.0);
}

#[tokio::test]
async fn test_order_book_empty() {
    let book = OrderBook::new("BTCUSDT".to_string());

    // Empty book should return None for best bid/ask
    assert!(book.best_bid().is_none());
    assert!(book.best_ask().is_none());
}

#[tokio::test]
async fn test_concurrent_book_updates() {
    let books = Arc::new(RwLock::new(HashMap::new()));

    // Insert a book
    {
        let mut books_write = books.write().await;
        books_write.insert("BTCUSDT".to_string(), OrderBook::new("BTCUSDT".to_string()));
    }

    // Create a bunch of update tasks
    let mut tasks = Vec::new();
    for i in 0..10 {
        let books_clone = Arc::clone(&books);
        let task = task::spawn(async move {
            let price_offset = i as f64 * 10.0;
            let quote = Quote {
                symbol: "BTCUSDT".to_string(),
                bid: 50000.0 - price_offset,
                ask: 50010.0 + price_offset,
                bid_size: 1.0,
                ask_size: 1.0,
                venue: "TEST".to_string(),
                timestamp: 0,
            };

            let mut books_write = books_clone.write().await;
            let book = books_write.get_mut("BTCUSDT").unwrap();
            book.update(&quote);
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        task.await.unwrap();
    }

    // Check the book state
    let books_read = books.read().await;
    let book = books_read.get("BTCUSDT").unwrap();

    // Book should have 10 levels
    assert_eq!(book.bids.len(), 10);
    assert_eq!(book.asks.len(), 10);

    // Best bid should be 50000.0
    let (bid_price, _) = book.best_bid().unwrap();
    assert_eq!(bid_price, 50000.0);

    // Best ask should be 50010.0
    let (ask_price, _) = book.best_ask().unwrap();
    assert_eq!(ask_price, 50010.0);
}

#[tokio::test]
async fn test_extreme_price_values() {
    let mut book = OrderBook::new("BTCUSDT".to_string());

    // Test with very small prices
    let quote1 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 0.00000001,
        ask: 0.00000002,
        bid_size: 1.0,
        ask_size: 1.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote1);

    let (bid_price, _) = book.best_bid().unwrap();
    let (ask_price, _) = book.best_ask().unwrap();

    assert_eq!(bid_price, 0.00000001);
    assert_eq!(ask_price, 0.00000002);

    // Test with very large prices
    let quote2 = Quote {
        symbol: "BTCUSDT".to_string(),
        bid: 1_000_000.0,
        ask: 1_000_001.0,
        bid_size: 1.0,
        ask_size: 1.0,
        venue: "TEST".to_string(),
        timestamp: 0,
    };

    book.update(&quote2);

    let (bid_price, _) = book.best_bid().unwrap();
    let (ask_price, _) = book.best_ask().unwrap();

    assert_eq!(bid_price, 1_000_000.0);
    assert_eq!(ask_price, 1_000_001.0);
}
}
