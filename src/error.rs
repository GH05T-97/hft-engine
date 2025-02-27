use std::fmt;
use thiserror::Error;

/// Core error types for the HFT engine
#[derive(Error, Debug)]
pub enum HftError {
    #[error("Venue error: {0}")]
    Venue(#[from] VenueError),

    #[error("Gateway error: {0}")]
    Gateway(#[from] GatewayError),

    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Book error: {0}")]
    Book(#[from] BookError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Errors related to venue connections and operations
#[derive(Error, Debug)]
pub enum VenueError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Order submission failed: {0}")]
    OrderSubmissionFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Errors related to gateway operations
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("No venues configured")]
    NoVenuesConfigured,

    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("Channel capacity exceeded")]
    ChannelCapacityExceeded,

    #[error("Venue not found: {0}")]
    VenueNotFound(String),

    #[error("Failed to send data through channel: {0}")]
    ChannelSendFailed(String),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Gateway not running")]
    NotRunning,
}

/// Errors related to execution engine
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Invalid order: {0}")]
    InvalidOrder(String),

    #[error("Order rejected: {0}")]
    OrderRejected(String),

    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),
}

/// Errors related to order book operations
#[derive(Error, Debug)]
pub enum BookError {
    #[error("Invalid price: {0}")]
    InvalidPrice(f64),

    #[error("Invalid size: {0}")]
    InvalidSize(f64),

    #[error("Invalid book state")]
    InvalidBookState,
}

// Context wrapper to add context to errors
pub struct ErrorContext<E> {
    pub error: E,
    pub context: String,
}

impl<E: std::error::Error> fmt::Display for ErrorContext<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.error)
    }
}

impl<E: std::error::Error> fmt::Debug for ErrorContext<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {{ context: {}, error: {:?} }}", self.context, self.error)
    }
}

// Extension trait to add context to errors
pub trait ErrorExt<T, E> {
    fn context(self, context: &str) -> Result<T, ErrorContext<E>>;
}

impl<T, E: std::error::Error> ErrorExt<T, E> for Result<T, E> {
    fn context(self, context: &str) -> Result<T, ErrorContext<E>> {
        self.map_err(|error| ErrorContext {
            error,
            context: context.to_string(),
        })
    }
}