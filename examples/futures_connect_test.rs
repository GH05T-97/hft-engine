use hft_engine::venues::{BinanceVenue, VenueAdapter};  // Add VenueAdapter here
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let venue = BinanceVenue::new(
        String::new(),  // empty api key
        String::new(),  // empty secret
    );

    venue.subscribe_quotes(vec!["btcusdt".to_string()]).await?;

    println!("Connected and listening for quotes...");

    // Keep the program running
    tokio::signal::ctrl_c().await?;
    Ok(())
}