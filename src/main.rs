use std::sync::Arc;
use tokio::sync::RwLock;
use hft_engine::{
    services::Services,
    command::CommandControl,
    venues::binance::BinanceVenue,
};
use warp::Filter;
use prometheus::{gather, Encoder, TextEncoder};
use hft_engine::metrics;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize everything
    let mut services = Services::new().await;

    // Add venues
    let venue = Arc::new(BinanceVenue::new(
        std::env::var("BINANCE_API_KEY").unwrap_or_default(),
        std::env::var("BINANCE_API_SECRET").unwrap_or_default(),
    ));

    // Initialize command & control
    let services_arc = Arc::new(RwLock::new(services));
    let command_control = CommandControl::new(Arc::clone(&services_arc)).await;

    // Start trading
    command_control.start_trading().await?;

    println!("HFT Engine started successfully");

    // Keep the main thread running
    tokio::signal::ctrl_c().await?;  // Wait for Ctrl+C signal

    println!("Shutting down HFT Engine");
    Ok(())
}