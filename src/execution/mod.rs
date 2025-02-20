use tokio::sync::mpsc;
use crate::types::Order;

pub struct ExecutionEngine {
    pub(crate) order_tx: mpsc::Sender<Order>,
}