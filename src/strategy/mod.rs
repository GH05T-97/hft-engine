use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use crate::book::OrderBook;
use crate::types::Order;

pub struct Strategy {
    pub(crate) books: Arc<RwLock<HashMap<String, OrderBook>>>,
    pub(crate) order_tx: mpsc::Sender<Order>,
}