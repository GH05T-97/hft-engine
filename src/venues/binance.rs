use crate::types::{Order, Quote};
use crate::venues::VenueAdapter;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{
    connect_async,
    tungstenite::http::Request,
};
use url::Url;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct BinanceVenue {
    ws_url: String,
    api_key: String,
    api_secret: String,
    rest_url: String,
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
        }
    }

    async fn connect_websocket(&self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@bookTicker", s.to_lowercase()))
            .collect();

        let ws_url = format!("{}/{}", self.ws_url, streams.join("/"));
        println!("Connecting to: {}", ws_url);

        // Create a request instead of using URL directly
        let request = Request::builder()
            .uri(ws_url)
            .header("User-Agent", "Mozilla/5.0")
            .body(())?;

        let (ws_stream, _) = connect_async(request).await?;
        println!("WebSocket connected successfully");

        let (write, read) = ws_stream.split();

        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut read = read;

            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        println!("Raw message: {}", msg.to_string());
                        if let Ok(ticker) = serde_json::from_str::<BinanceBookTicker>(&msg.to_string()) {
                            let quote = Quote {
                                symbol: ticker.symbol,
                                bid: ticker.best_bid_price.parse().unwrap_or(0.0),
                                ask: ticker.best_ask_price.parse().unwrap_or(0.0),
								bid_size: ticker.best_bid_quantity.parse().unwrap_or(0.0),
								ask_size: ticker.best_ask_quantity.parse().unwrap_or(0.0),
                                venue: "BINANCE_FUTURES".to_string(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64,
                            };
                            println!("Processed quote: {:?}", quote);
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
    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), Box<dyn Error>> {
        self.connect_websocket(symbols).await
    }

	async fn submit_order(&self, order: Order) -> Result<String, Box<dyn Error>> {
		// TODO: Implement actual order submission
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