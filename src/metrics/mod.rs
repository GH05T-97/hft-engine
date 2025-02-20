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

// Add this to execution/mod.rs
use crate::metrics::{ORDER_LATENCY, ACTIVE_ORDERS};
use std::time::Instant;

impl ExecutionEngine {
    async fn execute_order(&self, order: Order) {
        let start = Instant::now();

        // Order execution logic here

        let duration = start.elapsed();
        ORDER_LATENCY
            .with_label_values(&[&order.venue, &order.order_type.to_string()])
            .observe(duration.as_secs_f64());

        ACTIVE_ORDERS
            .with_label_values(&[&order.venue])
            .inc();
    }
}