use lazy_static::lazy_static;
use prometheus::{register_histogram_vec, register_counter_vec, register_gauge_vec};
use prometheus::{HistogramVec, CounterVec, GaugeVec};

lazy_static! {
    pub static ref ORDER_LATENCY: HistogramVec = register_histogram_vec!(
        "hft_order_latency_seconds",
        "Order execution latency in seconds",
        &["venue", "order_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap();

    pub static ref ORDERBOOK_UPDATES: CounterVec = register_counter_vec!(
        "hft_orderbook_updates_total",
        "Total number of orderbook updates",
        &["symbol"]
    ).unwrap();

    pub static ref ACTIVE_ORDERS: GaugeVec = register_gauge_vec!(
        "hft_active_orders",
        "Number of active orders",
        &["venue"]
    ).unwrap();
}

async fn metrics_handler() -> Result<impl warp::Reply, warp::Rejection> {
    let encoder = prometheus::TextEncoder::new();
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