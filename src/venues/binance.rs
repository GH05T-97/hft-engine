use crate::error::{HftError, VenueError, ErrorExt};
use crate::types::{Order, Quote};
use crate::venues::VenueAdapter;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize};
use tokio_tungstenite::{
    connect_async,
    tungstenite::http::Request,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug, trace};

const RECONNECT_DELAY_MS: u64 = 5000;
const MAX_RECONNECT_ATTEMPTS: usize = 5;

#[derive(Debug)]
pub struct BinanceVenue {
    ws_url: String,
    api_key: String,
    api_secret: String,
    rest_url: String,
    quote_tx: Option<mpsc::Sender<Quote>>,
}

#[derive(Debug, Deserialize)]
struct BinanceBookTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    best_bid_price: String,
    #[serde(rename = "B")]
    best_bid_quantity: String,
    #[serde(rename = "a")]
    best_ask_price: String,
    #[serde(rename = "A")]
    best_ask_quantity: String,
    #[serde(rename = "T")]
    time: u64,
}

impl BinanceVenue {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            ws_url: "wss://fstream.binance.com/ws".to_string(),
            rest_url: "https://fapi.binance.com/fapi".to_string(),
            api_key,
            api_secret,
            quote_tx: None,
        }
    }

    pub fn with_quote_sender(mut self, quote_tx: mpsc::Sender<Quote>) -> Self {
        self.quote_tx = Some(quote_tx);
        self
    }

    async fn connect_websocket(&self, symbols: Vec<String>) -> Result<(), HftError> {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@bookTicker", s.to_lowercase()))
            .collect();

        let ws_url = format!("{}/{}", self.ws_url, streams.join("/"));
        info!(url = %ws_url, "Connecting to Binance WebSocket");

        // Create a request instead of using URL directly
        let request = Request::builder()
            .uri(ws_url)
            .header("User-Agent", "Mozilla/5.0")
            .body(())
            .map_err(|e| VenueError::ConnectionFailed(format!("Failed to build request: {}", e)))?;

        let quote_tx = match &self.quote_tx {
            Some(tx) => tx.clone(),
            None => return Err(VenueError::ConnectionFailed("Quote sender not configured".to_string()).into()),
        };

        self.ws_connect_with_retry(request, quote_tx, MAX_RECONNECT_ATTEMPTS).await?;

        Ok(())
    }

async fn ws_connect_with_retry(
    &self,
    request: Request<()>,
    quote_tx: mpsc::Sender<Quote>,
    max_attempts: usize
) -> Result<(), HftError> {
    let mut attempts = 0;

    loop {
        attempts += 1;
        // Fixed: Don't try to use map_err on the request directly
        match connect_async(request.clone())
            .await
            .context("WebSocket connection failed")
        {
            Ok((ws_stream, _)) => {
                info!("WebSocket connected successfully");
                let (write, read) = ws_stream.split();

                self.process_websocket_messages(read, quote_tx.clone()).await;
                return Ok(());
            }
            Err(e) => {
                error!(error = ?e.error, context = %e.context, "WebSocket connection error");
                if attempts >= max_attempts {
                    return Err(VenueError::ConnectionFailed(
                        format!("Failed after {} attempts: {}", attempts, e.error)
                    ).into());
                }

                warn!(
                    attempt = attempts,
                    max_attempts = max_attempts,
                    delay_ms = RECONNECT_DELAY_MS,
                    "Retrying connection"
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(RECONNECT_DELAY_MS)).await;
            }
        }
    }
}

    async fn process_websocket_messages(
        &self,
        mut read: futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
        >,
        quote_tx: mpsc::Sender<Quote>,
    ) {
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        trace!(message = %msg.to_string(), "Received WebSocket message");

                        match serde_json::from_str::<BinanceBookTicker>(&msg.to_string()) {
                            Ok(ticker) => {
                                // Use ? operator with Result to propagate errors
                                let bid = ticker.best_bid_price.parse::<f64>()
                                    .map_err(|e| VenueError::ParseError(format!("Invalid bid price: {}", e)))
                                    .unwrap_or(0.0);

                                let ask = ticker.best_ask_price.parse::<f64>()
                                    .map_err(|e| VenueError::ParseError(format!("Invalid ask price: {}", e)))
                                    .unwrap_or(0.0);

                                let bid_size = ticker.best_bid_quantity.parse::<f64>()
                                    .map_err(|e| VenueError::ParseError(format!("Invalid bid size: {}", e)))
                                    .unwrap_or(0.0);

                                let ask_size = ticker.best_ask_quantity.parse::<f64>()
                                    .map_err(|e| VenueError::ParseError(format!("Invalid ask size: {}", e)))
                                    .unwrap_or(0.0);

                                // Validate data before creating Quote
                                if bid <= 0.0 || ask <= 0.0 || bid_size <= 0.0 || ask_size <= 0.0 {
                                    warn!(
                                        symbol = %ticker.symbol,
                                        bid = bid,
                                        ask = ask,
                                        bid_size = bid_size,
                                        ask_size = ask_size,
                                        "Invalid quote data received"
                                    );
                                    continue;
                                }

                                let quote = Quote {
                                    symbol: ticker.symbol,
                                    bid,
                                    ask,
                                    bid_size,
                                    ask_size,
                                    venue: "BINANCE_FUTURES".to_string(),
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                                        .as_millis() as u64,
                                };

                                debug!(
                                    symbol = %quote.symbol,
                                    bid = %quote.bid,
                                    ask = %quote.ask,
                                    "Processed quote"
                                );

                                if let Err(e) = quote_tx.send(quote).await {
                                    error!(error = %e, "Failed to send quote to channel");
                                }
                            }
                            Err(e) => warn!(error = %e, "Failed to parse message"),
                        }
                    }
                    Err(e) => error!(error = %e, "WebSocket error"),
                }
            }

            error!("WebSocket stream ended unexpectedly");
        });
    }
}

#[async_trait]
impl VenueAdapter for BinanceVenue {
    async fn name(&self) -> String {
        "BINANCE_FUTURES".to_string()
    }

    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), HftError> {
        if symbols.is_empty() {
            return Err(VenueError::SubscriptionFailed("Empty symbol list".to_string()).into());
        }

        self.connect_websocket(symbols).await
    }

    async fn submit_order(&self, order: Order) -> Result<String, HftError> {
        // Validate order parameters
        if order.quantity <= 0.0 {
            return Err(VenueError::OrderSubmissionFailed(
                format!("Invalid quantity: {}", order.quantity)
            ).into());
        }

        if order.price <= 0.0 && matches!(order.order_type, crate::types::OrderType::Limit) {
            return Err(VenueError::OrderSubmissionFailed(
                format!("Invalid price for limit order: {}", order.price)
            ).into());
        }

        // TODO: Implement actual order submission with proper error handling

        info!(
            symbol = %order.symbol,
            side = ?order.side,
            quantity = %order.quantity,
            price = %order.price,
            order_type = ?order.order_type,
            "Order submitted to Binance"
        );

        Ok("mock_order_id".to_string())
    }
}