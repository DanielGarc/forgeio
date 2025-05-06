use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;

/// Represents the quality of a tag's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Quality {
    Good,
    Uncertain,
    Bad,
    Initializing,
    CommFailure, // Specific bad quality
    ConfigError, // Specific bad quality
}

impl Default for Quality {
    fn default() -> Self {
        Quality::Initializing
    }
}

/// Represents the value, quality, and timestamp of a tag.
#[derive(Debug, Clone)]
pub struct TagValue {
    pub value: ValueVariant,
    pub quality: Quality,
    pub timestamp: u64, // Unix timestamp milliseconds
}

impl TagValue {
    // Helper to create a new TagValue with current time
    pub fn new(value: ValueVariant, quality: Quality) -> Self {
        TagValue {
            value,
            quality,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    // Helper for bad quality
    pub fn bad(reason: Quality) -> Self {
        Self::new(ValueVariant::Null, reason)
    }
}

/// Possible data types for a tag's value.
#[derive(Debug, Clone, PartialEq)] // Add PartialEq for comparisons
pub enum ValueVariant {
    Null, // Representing no value or initial state
    Bool(bool),
    Int(i64),
    UInt(u64), // Added unsigned int
    Float(f64),
    String(String),
    // TODO: Add complex types: Array, Struct/Object
}

/// Represents a single tag in the system.
#[derive(Debug, Clone)]
pub struct Tag {
    /// Unique path identifying the tag (e.g., "Folder/Device/TagName").
    pub path: String,
    /// Current value, quality, and timestamp.
    pub value: TagValue,
    /// Source driver ID providing this tag's value.
    pub driver_id: String,
    /// Protocol-specific address for this tag on the source device.
    pub driver_address: String,
    /// Poll rate in milliseconds.
    pub poll_rate_ms: u64,
    /// Metadata about the tag.
    pub metadata: TagMetadata,
}

/// Metadata associated with a tag.
#[derive(Debug, Clone, Default)] // Default trait for easy initialization
pub struct TagMetadata {
    pub description: Option<String>,
    pub eng_unit: Option<String>,
    pub eng_low: Option<f64>,
    pub eng_high: Option<f64>,
    pub writable: bool,
    // Add other relevant metadata: security, history settings etc.
}
