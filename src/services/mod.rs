use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;

use crate::gateways::{quote::QuoteGateway, order::OrderGateway};
use crate::book::{BookBuilder, OrderBook};
use crate::strategy::Strategy;
use crate::execution::ExecutionEngine;
use crate::venues::BinanceVenue;

pub struct Services {
    quote_gateway: QuoteGateway,
    order_gateway: OrderGateway,
    book_builder: BookBuilder,
    strategy: Strategy,
    execution: ExecutionEngine,
}

impl Services {
    pub async fn new() -> Self {
        let (quote_tx, quote_rx) = mpsc::channel(1000);
        let (order_tx, order_rx) = mpsc::channel(1000);
        let books = Arc::new(RwLock::new(HashMap::new()));

        let binance = Arc::new(BinanceVenue::new(
            std::env::var("BINANCE_API_KEY").unwrap_or_default(),
            std::env::var("BINANCE_API_SECRET").unwrap_or_default(),
        ));

        Self {
            quote_gateway: QuoteGateway {
                venues: Vec::new(),
                quote_tx,
            },
            order_gateway: OrderGateway {
                venues: Vec::new(),
                order_rx,
            },
            book_builder: BookBuilder {
                books: Arc::clone(&books),
                quote_rx,
            },
            strategy: Strategy {
                books: Arc::clone(&books),
                order_tx: order_tx.clone(),
            },
            execution: ExecutionEngine {
                order_tx,
            },
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start all components
        Ok(())
    }
}