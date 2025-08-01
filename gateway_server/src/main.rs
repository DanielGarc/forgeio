use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use gateway_server::config::settings::Settings;
use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, TagRequest};
use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Quality, Tag, TagMetadata, TagValue};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration, Instant};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::validate_request::ValidateRequestHeaderLayer;

// Modules are defined in the accompanying library crate (lib.rs)

// Potentially other modules like scripting, historian, events etc.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ForgeIO Gateway Server starting...");

    // --- Load Configuration ---
    let config_path = Path::new("config.toml");
    let settings = match Settings::load(config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "FATAL: Failed to load configuration from {:?}: {}",
                config_path, e
            );
            std::process::exit(1);
        }
    };
    println!(
        "Configuration loaded: {} devices, {} tags",
        settings.devices.len(),
        settings.tags.len()
    );

    // --- Initialize Tag Engine ---
    let tag_engine = TagEngine::new();
    let tag_engine_arc = Arc::new(tag_engine); // Wrap in Arc for sharing
    println!("Tag Engine initialized.");

    // --- Initialize Drivers ---
    // Store drivers in a thread-safe way, accessible by ID
    let mut driver_instances: HashMap<String, Arc<Mutex<dyn DeviceDriver + Send + Sync>>> =
        HashMap::new();

    for driver_config in settings.devices {
        println!(
            "Initializing driver: {} ({})",
            driver_config.name, driver_config.id
        );

        // TODO: Add a 'driver_type' field to DriverConfig to select the correct driver
        // For now, assume all are OPC UA if opcua driver exists
        let driver: Arc<Mutex<dyn DeviceDriver + Send + Sync>> = {
            // Create the specific driver instance
            let mut opcua_driver = OpcUaDriver::new(driver_config.clone())
                .map_err(|e| format!("Failed to create OPC UA driver: {}", e))?;
            // Attempt to connect
            opcua_driver
                .connect()
                .map_err(|e| format!("Failed to connect OPC UA driver: {}", e))?;
            Arc::new(Mutex::new(opcua_driver)) // Wrap in Arc<Mutex>
        };

        driver_instances.insert(driver_config.id.clone(), driver);
    }
    let drivers_arc = Arc::new(driver_instances); // Share the driver map
    println!("{} drivers initialized and connected.", drivers_arc.len());

    // --- Register Tags ---
    for tag_config in settings.tags {
        // Check if the driver for this tag exists and was initialized
        if drivers_arc.contains_key(&tag_config.driver_id) {
            println!(
                "Registering tag: {} (Driver: {}, Address: {}, Rate: {}ms)",
                tag_config.path, tag_config.driver_id, tag_config.address, tag_config.poll_rate_ms
            );

            let metadata = TagMetadata {
                description: Some("Default description".to_string()),
                eng_unit: Some("unit".to_string()),
                eng_low: Some(f64::MIN),
                eng_high: Some(f64::MAX),
                writable: false, // Ensure all fields are correctly set
            };

            let initial_tag = Tag {
                path: tag_config.path.clone(),
                value: TagValue::bad(Quality::Bad), // Start with Bad quality
                driver_id: tag_config.driver_id.clone(),
                driver_address: tag_config.address.clone(),
                poll_rate_ms: tag_config.poll_rate_ms,
                metadata, // Basic metadata
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
            if let Some(tag) = polling_tag_engine.get_tag_details(&tag_path) {
                // Assumed method
                poll_groups
                    .entry((tag.driver_id.clone(), tag.poll_rate_ms))
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
                let last_poll = last_poll_times
                    .entry((driver_id.clone(), *poll_rate_ms))
                    .or_insert(Instant::now() - Duration::from_secs(60));

                if now.duration_since(*last_poll) >= poll_duration {
                    // This group is due for polling
                    println!(
                        "Polling group: Driver '{}', Rate {}ms, Tags: {}",
                        driver_id,
                        poll_rate_ms,
                        tag_paths.len()
                    );

                    if let Some(driver_mutex) = polling_drivers.get(driver_id) {
                        let mut requests = Vec::new();
                        // Need tag address again - requires TagEngine modification or storing more info
                        for path in tag_paths {
                            if let Some(tag) = polling_tag_engine.get_tag_details(path) {
                                // Assumed method
                                requests.push(TagRequest {
                                    address: tag.driver_address,
                                });
                            }
                        }

                        if !requests.is_empty() {
                            let mut driver = driver_mutex.lock().await; // Lock the driver for the read call
                            match driver.read_tags(&requests).await {
                                Ok(results) => {
                                    println!(
                                        "Read successful for {} tags from driver '{}'",
                                        results.len(),
                                        driver_id
                                    );
                                    for (address, driver_tag_value) in results {
                                        if let Some(path) = polling_tag_engine
                                            .find_path_by_address(driver_id, &address)
                                        {
                                            let structures_tag_value =
                                                TagValue::from(driver_tag_value);
                                            polling_tag_engine
                                                .update_tag_value(&path, structures_tag_value);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "ERROR: Failed to read tags from driver '{}': {}",
                                        driver_id, e
                                    );
                                    // Optionally mark tags as Bad quality
                                    for path in tag_paths {
                                        polling_tag_engine
                                            .update_tag_value(path, TagValue::bad(Quality::Bad));
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
        .route("/api/health", get(root))
        .route("/tags", get(get_tags)) // New route for tags
        .with_state(tag_engine_arc)
        .fallback_service(
            ServeDir::new("webui/dist").not_found_service(ServeFile::new("webui/dist/index.html")),
        )
        // Serve frontend and support client-side routing
        .layer(ValidateRequestHeaderLayer::basic("admin", "admin"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Simple health check endpoint
async fn root() -> &'static str {
    "ForgeIO Gateway Server Running"
}

async fn get_tags(State(tag_engine): State<Arc<TagEngine>>) -> impl IntoResponse {
    let tags = tag_engine.get_all_tags().await;
    Json(json!(tags))
}
