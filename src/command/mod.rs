use std::sync::Arc;
use tokio::sync::RwLock;
use crate::services::Services;

pub struct CommandControl {
    services: Arc<RwLock<Services>>,
}

impl CommandControl {
    pub async fn new(services: Arc<RwLock<Services>>) -> Self {
        Self { services }
    }

    pub async fn start_trading(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut services = self.services.write().await;
        services.start().await
    }
}