use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gateway_server::api::rest::{create_api_routes, SharedAppState};
use gateway_server::config::settings::Settings;
use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Tag, TagValue, ValueVariant, TagMetadata, Quality};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tower::ServiceExt;
use axum::Router;

fn create_test_tag_engine() -> Arc<TagEngine> {
    let engine = Arc::new(TagEngine::new());
    
    // Add some test tags
    let test_tag = Tag {
        path: "TestDevice/Temperature".to_string(),
        value: TagValue::new(ValueVariant::Float(23.5), Quality::Good),
        driver_id: "test_driver".to_string(),
        driver_address: "test_addr".to_string(),
        poll_rate_ms: 1000,
        metadata: TagMetadata::default(),
    };
    engine.register_tag(test_tag);
    
    engine
}

fn create_test_app_state() -> SharedAppState {
    let engine = create_test_tag_engine();
    let settings = Settings {
        devices: vec![],
        tags: vec![],
    };
    
    SharedAppState {
        tag_engine: engine,
        driver_count: 0,
        start_time: Instant::now(),
        settings: Arc::new(RwLock::new(settings)),
        drivers: Arc::new(HashMap::new()),
    }
}

fn create_test_app() -> Router {
    let state = create_test_app_state();
    create_api_routes().with_state(state)
}

#[tokio::test]
async fn test_opcua_discover_endpoint() {
    let app = create_test_app();

    let request = Request::builder()
        .uri("/api/opcua/discover")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_opcua_browse_nonexistent_driver() {
    let app = create_test_app();

    let request = Request::builder()
        .uri("/api/opcua/browse/nonexistent_driver")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_opcua_discover_tags_nonexistent_driver() {
    let app = create_test_app();

    let request = Request::builder()
        .uri("/api/opcua/discover-tags/nonexistent_driver")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
