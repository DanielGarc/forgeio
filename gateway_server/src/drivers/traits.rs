use crate::tags::structures::TagValue;
use async_trait::async_trait;
use serde::{Deserialize, Serialize}; // Added for config
use std::any::Any;
use std::collections::HashMap;
use std::error::Error; // Imported from structures to avoid duplication

/// Configuration for an OPC UA driver
#[derive(Debug, Clone, Deserialize, Serialize)] // Added Deserialize, Serialize, and Debug
pub struct OpcDriverConfig {
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
pub struct OpcTagRequest {
    pub address: String, // Protocol-specific tag address (e.g., "ns=1;s=MyTag", "40001", "Topic/Subtopic")
                         // Potentially add data type hint
}

// Type alias for results from driver operations
pub type OpcDriverResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Trait implemented by OPC UA drivers.
#[async_trait]
pub trait OpcDriver: Send + Sync {
    /// Get the configuration of this driver instance.
    fn config(&self) -> &OpcDriverConfig;

    /// Connect to the underlying device.
    async fn connect(&self) -> OpcDriverResult<()>;

    /// Disconnect from the underlying device.
    async fn disconnect(&self) -> OpcDriverResult<()>;

    /// Check the connection status.
    async fn check_status(&self) -> OpcDriverResult<()>; // Returns Ok(()) if connected, Err otherwise

    /// Read a batch of tags.
    /// Takes a list of tag addresses and returns a map of address to TagValue.
    async fn read_tags(&self, tags: &[OpcTagRequest]) -> OpcDriverResult<HashMap<String, TagValue>>;

    /// Write a batch of tags.
    /// Takes a map of tag address to the TagValue to write.
    /// Returns a map of address to TagValue representing the result (e.g., success or error status per tag).
    async fn write_tags(
        &self,
        tags: HashMap<String, TagValue>,
    ) -> OpcDriverResult<HashMap<String, TagValue>>;

    /// Enable downcasting to concrete types
    fn as_any(&self) -> &dyn Any;

    // TODO: Add methods for subscription-based updates if the protocol supports it
    // async fn subscribe_tags(&mut self, tags: &[OpcTagRequest]) -> OpcDriverResult<()>;
    // async fn unsubscribe_tags(&mut self, tags: &[OpcTagRequest]) -> OpcDriverResult<()>;
    // Potentially return a stream or use a callback mechanism for subscription updates
}
