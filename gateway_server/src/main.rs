use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex; // Use tokio's Mutex for async locking
use tokio::time::{interval, Duration, Instant}; // For polling loop

// Core Modules
mod drivers;
mod tags;
mod config;
mod api;

// Use necessary types
use config::settings::{Settings, TagConfig}; // Import TagConfig
use tags::engine::TagEngine;
use tags::structures::{Tag, TagValue, TagMetadata}; // Import Tag and TagMetadata
use drivers::traits::{DeviceDriver, DriverConfig, TagRequest}; // Import DeviceDriver trait and TagRequest
use drivers::opcua::OpcUaDriver; // Import the specific driver implementation

// Potentially other modules like scripting, historian, events etc.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ForgeIO Gateway Server starting...");

    // --- Load Configuration ---
    let config_path = Path::new("config.toml");
    let settings = match Settings::load(config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FATAL: Failed to load configuration from {:?}: {}", config_path, e);
            std::process::exit(1);
        }
    };
    println!("Configuration loaded: {} devices, {} tags", settings.devices.len(), settings.tags.len());

    // --- Initialize Tag Engine ---
    let tag_engine = TagEngine::new();
    let tag_engine_arc = Arc::new(tag_engine); // Wrap in Arc for sharing
    println!("Tag Engine initialized.");

    // --- Initialize Drivers ---
    // Store drivers in a thread-safe way, accessible by ID
    let mut driver_instances: HashMap<String, Arc<Mutex<dyn DeviceDriver + Send + Sync>>> = HashMap::new();

    for driver_config in settings.devices {
        println!("Initializing driver: {} ({})", driver_config.name, driver_config.id);

        // TODO: Add a 'driver_type' field to DriverConfig to select the correct driver
        // For now, assume all are OPC UA if opcua driver exists
        let driver: Arc<Mutex<dyn DeviceDriver + Send + Sync>> = {
            // Create the specific driver instance
            let mut opcua_driver = OpcUaDriver::new(driver_config.clone());
            // Attempt to connect
            match opcua_driver.connect().await {
                Ok(_) => {
                    println!("Driver '{}' connected successfully.", driver_config.id);
                    Arc::new(Mutex::new(opcua_driver)) // Wrap in Arc<Mutex>
                }
                Err(e) => {
                    eprintln!("ERROR: Failed to connect driver '{}': {}. Skipping driver.", driver_config.id, e);
                    continue; // Skip this driver if connection failed
                }
            }
        };

        driver_instances.insert(driver_config.id.clone(), driver);
    }
    let drivers_arc = Arc::new(driver_instances); // Share the driver map
    println!("{} drivers initialized and connected.", drivers_arc.len());


    // --- Register Tags ---
    for tag_config in settings.tags {
        // Check if the driver for this tag exists and was initialized
        if drivers_arc.contains_key(&tag_config.driver_id) {
            println!("Registering tag: {} (Driver: {}, Address: {}, Rate: {}ms)",
                tag_config.path, tag_config.driver_id, tag_config.address, tag_config.poll_rate_ms);

            let initial_tag = Tag {
                path: tag_config.path.clone(),
                value: TagValue::bad("Initializing"), // Start with Bad quality
                driver_id: tag_config.driver_id.clone(),
                driver_address: tag_config.address.clone(),
                poll_rate_ms: tag_config.poll_rate_ms,
                metadata: TagMetadata { description: None }, // Basic metadata
            };
            tag_engine_arc.register_tag(initial_tag);
        } else {
             eprintln!("WARN: Skipping tag '{}' because its driver '{}' was not found or failed to initialize.",
                tag_config.path, tag_config.driver_id);
        }
    }
     println!("Tags registered in Tag Engine.");

    // --- Start Polling Loop ---
    let polling_tag_engine = Arc::clone(&tag_engine_arc);
    let polling_drivers = Arc::clone(&drivers_arc);

    tokio::spawn(async move {
        println!("Polling task started.");
        // Group tags by (driver_id, poll_rate_ms)
        let mut poll_groups: HashMap<(String, u64), Vec<String>> = HashMap::new();
        for tag_path in polling_tag_engine.get_all_tag_paths() {
             // We need the full Tag info here, not just the path.
             // Let's modify TagEngine slightly or fetch details here.
             // For now, assuming we can get Tag details from the path.
             // THIS IS A SIMPLIFICATION - requires TagEngine modification
             if let Some(tag) = polling_tag_engine.get_tag_details(&tag_path) { // Assumed method
                 poll_groups.entry((tag.driver_id.clone(), tag.poll_rate_ms))
                     .or_default()
                     .push(tag_path);
             }
        }
        println!("Polling groups created: {}", poll_groups.len());

        // Store last poll time for each group
        let mut last_poll_times: HashMap<(String, u64), Instant> = HashMap::new();
        let base_interval = Duration::from_millis(100); // Check every 100ms which groups are due
        let mut tick_interval = interval(base_interval);

        loop {
            tick_interval.tick().await;
            let now = Instant::now();

            for ((driver_id, poll_rate_ms), tag_paths) in &poll_groups {
                let poll_duration = Duration::from_millis(*poll_rate_ms);
                let last_poll = last_poll_times.entry((driver_id.clone(), *poll_rate_ms)).or_insert(Instant::MIN);

                if now.duration_since(*last_poll) >= poll_duration {
                    // This group is due for polling
                     println!("Polling group: Driver '{}', Rate {}ms, Tags: {}", driver_id, poll_rate_ms, tag_paths.len());

                    if let Some(driver_mutex) = polling_drivers.get(driver_id) {
                        let mut requests = Vec::new();
                        // Need tag address again - requires TagEngine modification or storing more info
                        for path in tag_paths {
                             if let Some(tag) = polling_tag_engine.get_tag_details(path) { // Assumed method
                                requests.push(TagRequest { address: tag.driver_address });
                             }
                        }

                        if !requests.is_empty() {
                            let mut driver = driver_mutex.lock().await; // Lock the driver for the read call
                            match driver.read_tags(&requests).await {
                                Ok(results) => {
                                    println!("Read successful for {} tags from driver '{}'", results.len(), driver_id);
                                    for (address, tag_value) in results {
                                        // Need to map address back to tag path - requires TagEngine modification or lookup map
                                        if let Some(path) = polling_tag_engine.find_path_by_address(driver_id, &address) { // Assumed method
                                             polling_tag_engine.update_tag_value(&path, tag_value);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("ERROR: Failed to read tags from driver '{}': {}", driver_id, e);
                                    // Optionally mark tags as Bad quality
                                    for path in tag_paths {
                                        polling_tag_engine.update_tag_value(path, TagValue::bad(&format!("Read Error: {}", e)));
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("WARN: Driver '{}' not found for polling.", driver_id);
                        // Mark tags as Bad?
                    }
                    // Update last poll time regardless of success/failure to avoid spamming logs on error
                    *last_poll = now;
                }
            }
        }
    });

    // --- Start API Server ---
    println!("Starting API server...");
    let app = Router::new()
        .route("/", get(root))
        // Pass tag engine state to handlers
        .with_state(tag_engine_arc); // Pass the Arc'd TagEngine

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Placeholder root handler
async fn root() -> &'static str {
    "ForgeIO Gateway Server Running"
}

// --- Helper methods needed in TagEngine (Assumed for now) ---
// NOTE: These methods need to be properly implemented in src/tags/engine.rs
impl TagEngine {
    // Assumed method to get full tag details - replace with actual implementation
     fn get_tag_details(&self, tag_path: &str) -> Option<Tag> {
         // Actual implementation needed - this is just a placeholder
         // It needs to access self.tags and return a CLONED Tag struct
         self.tags.get(tag_path).map(|tag_ref| tag_ref.value().clone())
         // ^^^ THIS IS INCORRECT - Needs to clone the whole Tag, not just TagValue ^^^ 
     }

    // Assumed method to find tag path by driver/address - replace with actual implementation
    fn find_path_by_address(&self, driver_id: &str, address: &str) -> Option<String> {
         // Actual implementation needed - iterate through self.tags
        self.tags.iter()
            .find(|entry| entry.driver_id == driver_id && entry.driver_address == address)
            .map(|entry| entry.key().clone())
    }
}
