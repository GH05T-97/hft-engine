use tokio::sync::mpsc;
use crate::types::Order;
use crate::metrics::{ORDER_LATENCY, ACTIVE_ORDERS};
use std::time::Instant;

pub struct ExecutionEngine {
    pub(crate) order_tx: mpsc::Sender<Order>,
}

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