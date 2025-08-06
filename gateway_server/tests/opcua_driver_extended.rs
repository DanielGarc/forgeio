use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, DriverConfig, TagRequest};
use tokio::time::{sleep, Duration};
use std::sync::Arc;

fn create_test_config(address: &str) -> DriverConfig {
    DriverConfig {
        id: "test_driver".into(),
        name: "Test OPC UA Driver".into(),
        address: address.into(),
        scan_rate_ms: 1000,
        application_name: Some("TestClient".into()),
        application_uri: None,
        session_name: Some("TestSession".into()),
        max_message_size: None,
        max_chunk_count: None,
        connect_retry_attempts: Some(3),
        connect_retry_delay_ms: Some(100),
        connect_retry_backoff: Some(1.5),
        connect_timeout_ms: Some(500),
    }
}

#[tokio::test]
async fn test_connection_failure() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Try to connect to a non-existent server
    let config = create_test_config("opc.tcp://127.0.0.1:9999/");
    let driver = OpcUaDriver::new(config).unwrap();
    
    // Connection should fail
    let result = driver.connect().await;
    assert!(result.is_err());
    
    // Status check should also fail
    let status = driver.check_status().await;
    assert!(status.is_err());
}

#[tokio::test]
async fn test_invalid_endpoint() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Try to connect to an invalid endpoint format
    let config = create_test_config("invalid://endpoint");
    let driver = OpcUaDriver::new(config).unwrap();
    
    let result = driver.connect().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_driver_configuration() {
    let config = DriverConfig {
        id: "test_id".into(),
        name: "Test Name".into(),
        address: "opc.tcp://127.0.0.1:4840/".into(),
        scan_rate_ms: 2000,
        application_name: Some("CustomApp".into()),
        application_uri: Some("urn:custom:app".into()),
        session_name: Some("CustomSession".into()),
        max_message_size: Some(1000000),
        max_chunk_count: Some(512),
        connect_retry_attempts: Some(10),
        connect_retry_delay_ms: Some(2000),
        connect_retry_backoff: Some(2.5),
        connect_timeout_ms: Some(5000),
    };
    
    let driver = OpcUaDriver::new(config.clone()).unwrap();
    let returned_config = driver.config();
    
    assert_eq!(returned_config.id, config.id);
    assert_eq!(returned_config.name, config.name);
    assert_eq!(returned_config.address, config.address);
    assert_eq!(returned_config.scan_rate_ms, config.scan_rate_ms);
    assert_eq!(returned_config.application_name, config.application_name);
    assert_eq!(returned_config.application_uri, config.application_uri);
    assert_eq!(returned_config.session_name, config.session_name);
    assert_eq!(returned_config.max_message_size, config.max_message_size);
    assert_eq!(returned_config.max_chunk_count, config.max_chunk_count);
    assert_eq!(returned_config.connect_retry_attempts, config.connect_retry_attempts);
    assert_eq!(returned_config.connect_retry_delay_ms, config.connect_retry_delay_ms);
    assert_eq!(returned_config.connect_retry_backoff, config.connect_retry_backoff);
    assert_eq!(returned_config.connect_timeout_ms, config.connect_timeout_ms);
}

#[tokio::test]
async fn test_read_tags_without_connection() {
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:4840/");
    let driver = OpcUaDriver::new(config).unwrap();
    
    // Try to read tags without connecting first
    let requests = vec![TagRequest {
        address: "ns=2;s=Temperature".to_string(),
    }];
    
    let result = driver.read_tags(&requests).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_browse_without_connection() {
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:4840/");
    let driver = OpcUaDriver::new(config).unwrap();
    
    // Try to browse without connecting first
    let result = driver.browse_node("ns=0;i=85").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_discover_tags_without_connection() {
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:4840/");
    let driver = OpcUaDriver::new(config).unwrap();
    
    // Try to discover tags without connecting first
    let result = driver.discover_tags().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_multiple_drivers() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create multiple driver instances
    let configs = vec![
        create_test_config("opc.tcp://127.0.0.1:4840/"),
        create_test_config("opc.tcp://127.0.0.1:4841/"),
        create_test_config("opc.tcp://127.0.0.1:4842/"),
    ];
    
    let mut drivers = Vec::new();
    for config in configs {
        let driver = OpcUaDriver::new(config).unwrap();
        drivers.push(Arc::new(driver));
    }
    
    // All should be created successfully
    assert_eq!(drivers.len(), 3);
    
    // Each should have different addresses
    assert_ne!(drivers[0].config().address, drivers[1].config().address);
    assert_ne!(drivers[1].config().address, drivers[2].config().address);
}

#[tokio::test]
async fn test_reconnection_logic() {
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:9999/"); // Non-existent server
    let driver = OpcUaDriver::new(config).unwrap();
    
    // First connection attempt should fail quickly due to our short timeout
    let start = std::time::Instant::now();
    let result = driver.connect().await;
    let duration = start.elapsed();
    
    assert!(result.is_err());
    // Should fail relatively quickly due to retry settings
    assert!(duration < Duration::from_secs(5));
}

#[tokio::test]
async fn test_invalid_node_ids() {
    // This test would require a working OPC UA server
    // For now, we'll test the error handling path
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:4840/");
    let driver = OpcUaDriver::new(config).unwrap();
    
    // These operations should fail gracefully with invalid node IDs
    let invalid_node_ids = vec![
        "invalid_node_id",
        "ns=999;s=NonExistent",
        "ns=-1;i=0",
        "",
    ];
    
    for node_id in invalid_node_ids {
        // Without connection, these should fail with connection errors
        // With connection to a real server, they should fail with invalid node errors
        let result = driver.browse_node(node_id).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_driver_lifecycle() {
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:4840/");
    let driver = OpcUaDriver::new(config).unwrap();
    
    // Check initial status (should be disconnected)
    let initial_status = driver.check_status().await;
    assert!(initial_status.is_err());
    
    // Try to connect (will fail without server, but should handle gracefully)
    let connect_result = driver.connect().await;
    assert!(connect_result.is_err());
    
    // Disconnect should complete without error even if not connected
    let disconnect_result = driver.disconnect().await;
    assert!(disconnect_result.is_ok());
    
    // Status should still show disconnected
    let final_status = driver.check_status().await;
    assert!(final_status.is_err());
}

#[tokio::test]
async fn test_concurrent_operations() {
    let _ = tracing_subscriber::fmt::try_init();
    
    let config = create_test_config("opc.tcp://127.0.0.1:4840/");
    let driver = Arc::new(OpcUaDriver::new(config).unwrap());
    
    let mut handles = vec![];
    
    // Spawn multiple concurrent operations
    for i in 0..5 {
        let driver_clone = Arc::clone(&driver);
        let handle = tokio::spawn(async move {
            // Each task tries different operations
            match i % 3 {
                0 => {
                    let _ = driver_clone.check_status().await;
                }
                1 => {
                    let _ = driver_clone.browse_node("ns=0;i=85").await;
                }
                2 => {
                    let requests = vec![TagRequest {
                        address: "ns=2;s=Test".to_string(),
                    }];
                    let _ = driver_clone.read_tags(&requests).await;
                }
                _ => {}
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    // Driver should still be in a consistent state
    let _ = driver.check_status().await;
}
