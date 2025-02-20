use super::VenueAdapter;
use crate::types::{Order, Quote};

pub struct ExampleVenue {
    pub venue_name: String,
}

#[async_trait]
impl VenueAdapter for ExampleVenue {
    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Subscribing to quotes for symbols: {:?} on {}", symbols, self.venue_name);
        Ok(())
    }

    async fn submit_order(&self, order: Order) -> Result<String, Box<dyn std::error::Error>> {
        println!("Submitting order to {}: {:?}", self.venue_name, order);
        Ok("order_id".to_string())
    }
}