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
        let services_clone = Arc::clone(&self.services);

        // Spawn the trading process in a background task
        tokio::spawn(async move {
            loop {
                let mut services = services_clone.write().await;
                if let Err(e) = services.start().await {
                    eprintln!("Error in trading loop: {}", e);
                    // Maybe add some retry logic or error handling
                }

                // Add a small delay before retrying
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        println!("Trading started successfully");
        Ok(())
    }

    pub async fn stop_trading(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Implement shutdown logic
        println!("Trading stopped");
        Ok(())
    }

    pub async fn status(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Implement status check
        Ok("Trading system running".to_string())
    }
}