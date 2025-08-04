use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::drivers::opcua::OpcUaDriver;
use crate::drivers::traits::DeviceDriver;
use crate::tags::engine::TagEngine;
use crate::config::settings::Settings;

#[derive(Clone)]
pub struct SharedAppState {
    pub tag_engine: Arc<TagEngine>,
    pub driver_count: usize,
    pub start_time: tokio::time::Instant,
    pub settings: Arc<RwLock<Settings>>,
    pub drivers: Arc<HashMap<String, Arc<dyn DeviceDriver + Send + Sync>>>,
}

#[derive(Deserialize)]
pub struct BrowseQuery {
    #[serde(default = "default_node_id")]
    node_id: String,
}

fn default_node_id() -> String {
    "ns=0;i=85".to_string() // Root Objects folder
}

#[derive(Serialize)]
pub struct BrowseResponse {
    pub node_id: String,
    pub children: Vec<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct DiscoverResponse {
    pub drivers: Vec<DriverInfo>,
}

#[derive(Serialize)]
pub struct DriverInfo {
    pub id: String,
    pub name: String,
    pub address: String,
    pub connected: bool,
    pub driver_type: String,
}

async fn discover_opcua_tags(
    State(state): State<SharedAppState>,
    Path(driver_id): Path<String>,
) -> impl IntoResponse {
    info!("Discovering OPC UA tags for driver: {}", driver_id);
    
    let driver = match state.drivers.get(&driver_id) {
        Some(driver) => driver,
        None => {
            warn!("Driver not found: {}", driver_id);
            return (
                StatusCode::NOT_FOUND,
                Json(TagDiscoveryResponse {
                    driver_id,
                    tags: vec![],
                    error: Some("Driver not found".to_string()),
                }),
            );
        }
    };

    // Try to downcast to OpcUaDriver to access discovery functionality
    let opcua_driver = driver.as_any().downcast_ref::<OpcUaDriver>();

    match opcua_driver {
        Some(opcua) => {
            match opcua.discover_tags().await {
                Ok(tags) => {
                    info!("Successfully discovered {} tags for driver {}", tags.len(), driver_id);
                    (
                        StatusCode::OK,
                        Json(TagDiscoveryResponse {
                            driver_id,
                            tags,
                            error: None,
                        }),
                    )
                }
                Err(e) => {
                    error!("Failed to discover tags for driver {}: {}", driver_id, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TagDiscoveryResponse {
                            driver_id,
                            tags: vec![],
                            error: Some(e.to_string()),
                        }),
                    )
                }
            }
        }
        None => {
            warn!("Driver '{}' is not an OPC UA driver", driver_id);
            (
                StatusCode::BAD_REQUEST,
                Json(TagDiscoveryResponse {
                    driver_id,
                    tags: vec![],
                    error: Some("Driver is not an OPC UA driver".to_string()),
                }),
            )
        }
    }
}

pub fn create_api_routes() -> Router<SharedAppState> {
    Router::new()
        .route("/api/opcua/browse/:driver_id", get(browse_opcua_tags))
        .route("/api/opcua/discover", get(discover_opcua_drivers))
        .route("/api/opcua/discover-tags/:driver_id", get(discover_opcua_tags))
}

async fn browse_opcua_tags(
    State(state): State<SharedAppState>,
    Path(driver_id): Path<String>,
    Query(params): Query<BrowseQuery>,
) -> impl IntoResponse {
    info!("Browsing OPC UA tags for driver: {}, node: {}", driver_id, params.node_id);
    
    let driver = match state.drivers.get(&driver_id) {
        Some(driver) => driver,
        None => {
            warn!("Driver not found: {}", driver_id);
            return (
                StatusCode::NOT_FOUND,
                Json(BrowseResponse {
                    node_id: params.node_id,
                    children: vec![],
                    error: Some(format!("Driver '{}' not found", driver_id)),
                }),
            );
        }
    };

    // Try to downcast to OpcUaDriver to access browse functionality
    let opcua_driver = driver.as_any().downcast_ref::<OpcUaDriver>();

    match opcua_driver {
        Some(opcua) => {
            match opcua.browse_node(&params.node_id).await {
                Ok(children) => {
                    info!("Successfully browsed {} children for node {}", children.len(), params.node_id);
                    (
                        StatusCode::OK,
                        Json(BrowseResponse {
                            node_id: params.node_id,
                            children,
                            error: None,
                        }),
                    )
                }
                Err(e) => {
                    error!("Failed to browse node {}: {}", params.node_id, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(BrowseResponse {
                            node_id: params.node_id,
                            children: vec![],
                            error: Some(e.to_string()),
                        }),
                    )
                }
            }
        }
        None => {
            warn!("Driver '{}' is not an OPC UA driver", driver_id);
            (
                StatusCode::BAD_REQUEST,
                Json(BrowseResponse {
                    node_id: params.node_id,
                    children: vec![],
                    error: Some(format!("Driver '{}' is not an OPC UA driver", driver_id)),
                }),
            )
        }
    }
}

async fn discover_opcua_drivers(State(state): State<SharedAppState>) -> impl IntoResponse {
    info!("Discovering OPC UA drivers");
    
    let mut drivers_info = Vec::new();
    
    for (id, driver) in state.drivers.iter() {
        let connected = driver.check_status().await.is_ok();
        let config = driver.config();
        
        // Check if it's an OPC UA driver
        let is_opcua = driver.as_any().downcast_ref::<OpcUaDriver>().is_some();
        
        drivers_info.push(DriverInfo {
            id: id.clone(),
            name: config.name.clone(),
            address: config.address.clone(),
            connected,
            driver_type: if is_opcua { "OPC UA".to_string() } else { "Unknown".to_string() },
        });
    }
    
    info!("Found {} drivers", drivers_info.len());
    (StatusCode::OK, Json(DiscoverResponse { drivers: drivers_info }))
}
