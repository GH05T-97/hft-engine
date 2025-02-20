use crate::types::{Order, Quote};
use crate::venues::VenueAdapter;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct BinanceVenue {
    ws_url: String,
    api_key: String,
    api_secret: String,
}

#[derive(Debug, Deserialize)]
struct BinanceBookTicker {
    symbol: String,
    best_bid_price: String,
    best_bid_quantity: String,
    best_ask_price: String,
    best_ask_quantity: String,
}

impl BinanceVenue {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            ws_url: "wss://stream.binance.com:9443/ws".to_string(),
            api_key,
            api_secret,
        }
    }

	async fn connect_websocket(&self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
		let streams: Vec<String> = symbols
			.iter()
			.map(|s| format!("{}@bookTicker", s.to_lowercase()))
			.collect();

		let ws_url = format!("{}/{}", self.ws_url, streams.join("/"));
		let url = Url::parse(&ws_url)?;

		let (ws_stream, _) = connect_async(url).await?;
		println!("WebSocket connected");

		let (write, read) = ws_stream.split();

		// Handle incoming messages
		tokio::spawn(async move {
			use futures_util::StreamExt;
			let mut read = read;

			while let Some(message) = read.next().await {
				match message {
					Ok(msg) => {
						if let Ok(ticker) = serde_json::from_str::<BinanceBookTicker>(&msg.to_string()) {
							// Convert to our Quote type and process
							let quote = Quote {
								symbol: ticker.symbol,
								bid: ticker.best_bid_price.parse().unwrap_or(0.0),
								ask: ticker.best_ask_price.parse().unwrap_or(0.0),
								venue: "BINANCE".to_string(),
								timestamp: std::time::SystemTime::now()
									.duration_since(std::time::UNIX_EPOCH)
									.unwrap()
									.as_millis() as u64,
							};
							println!("Received quote: {:?}", quote);
						}
					}
					Err(e) => println!("Error receiving message: {}", e),
				}
			}
		});

		Ok(())
	}
}

#[async_trait]
impl VenueAdapter for BinanceVenue {
    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        self.connect_websocket(symbols).await
    }

    async fn submit_order(&self, order: Order) -> Result<String, Box<dyn std::error::Error>> {
        // Implement order submission logic
        // For now, just log the order
        println!("Submitting order to Binance: {:?}", order);
        Ok("mock_order_id".to_string())
    }
}

// Add tests
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binance_subscription() {
        let venue = BinanceVenue::new(
            "fake_api_key".to_string(),
            "fake_api_secret".to_string(),
        );

        let symbols = vec!["BTCUSDT".to_string()];
        let result = venue.subscribe_quotes(symbols).await;
        assert!(result.is_ok());
    }
}