use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use gateway_server::api::rest::{create_api_routes, SharedAppState};
use gateway_server::config::settings::Settings;
use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, TagRequest};
use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Quality, Tag, TagMetadata, TagValue};
use gateway_server::logging::init_logging;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration, Instant};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::validate_request::ValidateRequestHeaderLayer;
use tracing::{error, info, warn};

// Modules are defined in the accompanying library crate (lib.rs)

// Potentially other modules like scripting, historian, events etc.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(None);
    info!("ForgeIO Gateway Server starting...");
    let start_time = Instant::now();

    // --- Load Configuration ---
    let config_path = Path::new("config.toml");
    let settings = match Settings::load(config_path) {
        Ok(s) => s,
        Err(e) => {
            error!(
                "FATAL: Failed to load configuration from {:?}: {}",
                config_path, e
            );
            std::process::exit(1);
        }
    };
    info!(
        "Configuration loaded: {} devices, {} tags",
        settings.devices.len(),
        settings.tags.len()
    );

    let settings_arc = Arc::new(RwLock::new(settings.clone()));

    // --- Initialize Tag Engine ---
    let tag_engine = TagEngine::new();
    let tag_engine_arc = Arc::new(tag_engine); // Wrap in Arc for sharing
    info!("Tag Engine initialized.");

    // --- Initialize Drivers ---
    // Store drivers in a thread-safe way, accessible by ID
    let mut driver_instances: HashMap<String, Arc<dyn DeviceDriver + Send + Sync>> = HashMap::new();

    for driver_config in settings.devices {
        info!(
            "Initializing driver: {} ({})",
            driver_config.name, driver_config.id
        );

        // TODO: Add a 'driver_type' field to DriverConfig to select the correct driver
        // For now, assume all are OPC UA if opcua driver exists
        let driver: Arc<dyn DeviceDriver + Send + Sync> = {
            let driver = Arc::new(
                OpcUaDriver::new(driver_config.clone())
                    .map_err(|e| format!("Failed to create OPC UA driver: {}", e))?,
            );
            driver
                .connect()
                .await
                .map_err(|e| format!("Failed to connect OPC UA driver: {}", e))?;
            driver
        };

        driver_instances.insert(driver_config.id.clone(), driver);
    }
    let drivers_arc = Arc::new(driver_instances); // Share the driver map
    info!("{} drivers initialized and connected.", drivers_arc.len());

    // --- Register Tags ---
    for tag_config in settings.tags {
        // Check if the driver for this tag exists and was initialized
        if drivers_arc.contains_key(&tag_config.driver_id) {
            info!(
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
            warn!("Skipping tag '{}' because its driver '{}' was not found or failed to initialize.",
                tag_config.path, tag_config.driver_id);
        }
    }
    info!("Tags registered in Tag Engine.");

    // --- Start Polling Loop ---
    let polling_tag_engine = Arc::clone(&tag_engine_arc);
    let polling_drivers = Arc::clone(&drivers_arc);

    tokio::spawn(async move {
        info!("Polling task started.");
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
        info!("Polling groups created: {}", poll_groups.len());

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
                    info!(
                        "Polling group: Driver '{}', Rate {}ms, Tags: {}",
                        driver_id,
                        poll_rate_ms,
                        tag_paths.len()
                    );

                    if let Some(driver) = polling_drivers.get(driver_id) {
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
                            match driver.read_tags(&requests).await {
                                Ok(results) => {
                                    info!(
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
                                    error!(
                                        "Failed to read tags from driver '{}': {}",
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
                        warn!("Driver '{}' not found for polling.", driver_id);
                        // Mark tags as Bad?
                    }
                    // Update last poll time regardless of success/failure to avoid spamming logs on error
                    *last_poll = now;
                }
            }
        }
    });

    // --- Start API Server ---
    info!("Starting API server...");
    let app_state = SharedAppState {
        tag_engine: Arc::clone(&tag_engine_arc),
        driver_count: drivers_arc.len(),
        start_time,
        settings: Arc::clone(&settings_arc),
        drivers: Arc::clone(&drivers_arc),
    };
    
    // Create the OPC UA API routes 
    let opcua_routes = create_api_routes();
    
    let app = Router::new()
        .route("/api/health", get(root))
        .route("/api/stats", get(stats))
        .route("/tags", get(get_tags))
        .route("/api/config", get(get_config).put(update_config))
        .merge(opcua_routes)
        .with_state(app_state)
        .fallback_service(
            ServeDir::new("webui/dist").not_found_service(ServeFile::new("webui/dist/index.html")),
        )
        // Serve frontend and support client-side routing
        .layer(ValidateRequestHeaderLayer::basic("admin", "admin"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Simple health check endpoint
async fn root() -> &'static str {
    "ForgeIO Gateway Server Running"
}

async fn get_tags(State(state): State<SharedAppState>) -> impl IntoResponse {
    let tags = state.tag_engine.get_all_tags().await;
    Json(json!(tags))
}

async fn get_config(State(state): State<SharedAppState>) -> impl IntoResponse {
    let cfg = state.settings.read().await.clone();
    Json(cfg)
}

async fn update_config(
    State(state): State<SharedAppState>,
    Json(new_cfg): Json<Settings>,
) -> impl IntoResponse {
    if let Err(e) = new_cfg.save(Path::new("config.toml")) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        );
    }
    let mut cfg_lock = state.settings.write().await;
    *cfg_lock = new_cfg;
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

async fn stats(State(state): State<SharedAppState>) -> impl IntoResponse {
    let tag_count = state.tag_engine.get_all_tag_paths().len();
    let uptime = state.start_time.elapsed().as_secs();
    Json(json!({
        "uptime_seconds": uptime,
        "tag_count": tag_count,
        "driver_count": state.driver_count,
    }))
}
