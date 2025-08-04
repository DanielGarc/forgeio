use crate::tags::structures::TagValue;
use async_trait::async_trait;
use serde::{Deserialize, Serialize}; // Added for config
use std::any::Any;
use std::collections::HashMap;
use std::error::Error; // Imported from structures to avoid duplication

/// Common configuration for all drivers
#[derive(Debug, Clone, Deserialize, Serialize)] // Added Deserialize, Serialize, and Debug
pub struct DriverConfig {
    pub id: String,        // Unique identifier for this device instance
    pub name: String,      // User-friendly name
    pub address: String,   // e.g., IP address, COM port, connection string
    pub scan_rate_ms: u64, // How often to poll tags (if applicable)
    // Additional optional OPC UA client parameters
    #[serde(default)]
    pub application_name: Option<String>,
    #[serde(default)]
    pub application_uri: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default)]
    pub max_message_size: Option<usize>,
    #[serde(default)]
    pub max_chunk_count: Option<usize>,
    #[serde(default)]
    pub connect_retry_attempts: Option<u32>,
    #[serde(default)]
    pub connect_retry_delay_ms: Option<u64>,
    #[serde(default)]
    pub connect_retry_backoff: Option<f64>,
    #[serde(default)]
    pub connect_timeout_ms: Option<u64>,
}

/// Represents a request to read or write a tag
#[derive(Clone)]
pub struct TagRequest {
    pub address: String, // Protocol-specific tag address (e.g., "ns=1;s=MyTag", "40001", "Topic/Subtopic")
                         // Potentially add data type hint
}

// Type alias for results from driver operations
pub type DriverResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// The core trait that all device protocol drivers must implement.
/// This allows the gateway to interact with different devices uniformly.
#[async_trait]
pub trait DeviceDriver: Send + Sync {
    /// Get the configuration of this driver instance.
    fn config(&self) -> &DriverConfig;

    /// Connect to the underlying device.
    async fn connect(&self) -> DriverResult<()>;

    /// Disconnect from the underlying device.
    async fn disconnect(&self) -> DriverResult<()>;

    /// Check the connection status.
    async fn check_status(&self) -> DriverResult<()>; // Returns Ok(()) if connected, Err otherwise

    /// Read a batch of tags.
    /// Takes a list of tag addresses and returns a map of address to TagValue.
    async fn read_tags(&self, tags: &[TagRequest]) -> DriverResult<HashMap<String, TagValue>>;

    /// Write a batch of tags.
    /// Takes a map of tag address to the TagValue to write.
    /// Returns a map of address to TagValue representing the result (e.g., success or error status per tag).
    async fn write_tags(
        &self,
        tags: HashMap<String, TagValue>,
    ) -> DriverResult<HashMap<String, TagValue>>;

    /// Enable downcasting to concrete types
    fn as_any(&self) -> &dyn Any;

    // TODO: Add methods for subscription-based updates if the protocol supports it
    // async fn subscribe_tags(&mut self, tags: &[TagRequest]) -> DriverResult<()>;
    // async fn unsubscribe_tags(&mut self, tags: &[TagRequest]) -> DriverResult<()>;
    // Potentially return a stream or use a callback mechanism for subscription updates
}
