use std::sync::Arc;
use tokio::sync::mpsc;
use crate::types::Order;
use crate::venues::VenueAdapter;

pub struct OrderGateway {
    pub(crate) venues: Vec<Arc<dyn VenueAdapter>>,
    pub(crate) order_rx: mpsc::Receiver<Order>,
}