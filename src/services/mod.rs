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
            quote_gateway: QuoteGateway::new(quote_tx),
            order_gateway: OrderGateway {
                venues: vec![],
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
        println!("Starting services...");

        // Start quote gateway
        println!("Starting quote gateway...");
        // Add your quote gateway start logic

        // Start order gateway
        println!("Starting order gateway...");
        // Add your order gateway start logic

        // Start book builder
        println!("Starting book builder...");
        // Add your book builder start logic

        // Start strategy
        println!("Starting strategy...");
        // Add your strategy start logic

        // Start execution engine
        println!("Starting execution engine...");
        // Add your execution engine start logic

        println!("All services started successfully");
        Ok(())
    }
}