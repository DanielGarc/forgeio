use crate::drivers::traits::DriverConfig; // Reuse DriverConfig for now
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct TagConfig {
    pub path: String,           // Unique path for the tag (e.g., "Folder/Sub/MyTag")
    pub driver_id: String,      // ID of the driver this tag belongs to (must match a device ID)
    pub address: String,        // Driver-specific address (e.g., OPC UA NodeId, Modbus register)
    pub poll_rate_ms: u64, // How often to poll this tag in milliseconds
                            // TODO: Add metadata, scaling, deadband etc. later
}

#[derive(Debug, Deserialize, Clone)] // Clone needed for passing around
pub struct Settings {
    // Maybe add general settings like server port, log level etc. later
    // pub server_port: u16,
    pub devices: Vec<DriverConfig>, // A list of device configurations
    #[serde(default)] // Make tags optional in the config file
    pub tags: Vec<TagConfig>,       // A list of tag configurations
}

impl Settings {
    pub fn load(config_path: &Path) -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start with defaults (optional)
            // .set_default("server_port", 3000)?
            // Add configuration file
            .add_source(File::from(config_path))
            // Add environment variables (optional, with prefix)
            // .add_source(Environment::with_prefix("APP"))
            .build()?;

        // Deserialize the entire configuration
        s.try_deserialize()
    }
}
