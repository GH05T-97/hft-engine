use hft_engine::venues::BinanceVenue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We don't need API keys for public market data
    let venue = BinanceVenue::new(
        String::new(),  // empty api key
        String::new(),  // empty secret
    );

    // Subscribe to BTCUSDT market data
    venue.subscribe_quotes(vec!["btcusdt".to_string()]).await?;

    println!("Connected and listening for quotes...");

    // Keep the program running
    tokio::signal::ctrl_c().await?;
    Ok(())
}