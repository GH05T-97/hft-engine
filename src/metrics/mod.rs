use lazy_static::lazy_static;
use prometheus::{register_histogram_vec, register_counter_vec, register_gauge_vec};
use prometheus::{HistogramVec, CounterVec, GaugeVec, Encoder, TextEncoder};
use warp::Filter;

lazy_static! {
    // Order execution metrics
    pub static ref ORDER_LATENCY: HistogramVec = register_histogram_vec!(
        "hft_order_latency_seconds",
        "Order execution latency in seconds",
        &["venue", "order_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap();

    // Order book metrics
    pub static ref ORDERBOOK_UPDATES: CounterVec = register_counter_vec!(
        "hft_orderbook_updates_total",
        "Total number of orderbook updates",
        &["symbol"]
    ).unwrap();

    // Order tracking metrics
    pub static ref ACTIVE_ORDERS: GaugeVec = register_gauge_vec!(
        "hft_active_orders",
        "Number of active orders",
        &["venue"]
    ).unwrap();

    // Quote gateway metrics
    pub static ref QUOTE_GATEWAY_THROUGHPUT: CounterVec = register_counter_vec!(
        "hft_quote_gateway_throughput_total",
        "Total number of quotes processed by the gateway",
        &["symbol", "venue"]
    ).unwrap();

    pub static ref QUOTE_GATEWAY_ERRORS: CounterVec = register_counter_vec!(
        "hft_quote_gateway_errors_total",
        "Total number of errors in the quote gateway",
        &["venue", "error_type"]
    ).unwrap();

    pub static ref QUOTE_LATENCY: HistogramVec = register_histogram_vec!(
        "hft_quote_latency_seconds",
        "Quote processing latency in seconds",
        &["venue", "symbol"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();

    // Venue metrics
    pub static ref VENUE_CONNECTIONS: GaugeVec = register_gauge_vec!(
        "hft_venue_connections",
        "Connection status for venues (1=connected, 0=disconnected)",
        &["venue"]
    ).unwrap();

    pub static ref VENUE_RECONNECTS: CounterVec = register_counter_vec!(
        "hft_venue_reconnects_total",
        "Total number of venue reconnection attempts",
        &["venue"]
    ).unwrap();
}

async fn metrics_handler() -> Result<impl warp::Reply, warp::Rejection> {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder.encode(&prometheus::gather(), &mut buffer).unwrap();

    Ok(warp::reply::with_header(
        String::from_utf8(buffer).unwrap(),
        "content-type",
        encoder.format_type(),
    ))
}

pub async fn init_metrics_server() {
    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .and_then(metrics_handler);

    println!("Starting metrics server on port 9090");

    tokio::spawn(warp::serve(metrics_route)
        .run(([0, 0, 0, 0], 9090)));
}