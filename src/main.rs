use std::sync::Arc;
use tokio::sync::RwLock;
use hft_engine::{
    services::Services,
    command::CommandControl,
    venues::example::ExampleVenue,
};
use warp::Filter;
use prometheus::{gather, Encoder, TextEncoder};
use hft_engine::metrics;

async fn metrics_handler() -> Result<impl warp::Reply, warp::Rejection> {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder.encode(&gather(), &mut buffer).unwrap();

    Ok(warp::reply::with_header(
        String::from_utf8(buffer).unwrap(),
        "content-type",
        encoder.format_type(),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    metrics::init_metrics_server().await;


    let mut services = Services::new().await;


    let venue1 = Arc::new(ExampleVenue {
        venue_name: "Venue1".to_string(),
    });

    let services_arc = Arc::new(RwLock::new(services));
    let command_control = CommandControl::new(Arc::clone(&services_arc)).await;

    command_control.start_trading().await?;

    Ok(())
}