use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Tag, TagValue, ValueVariant, TagMetadata, Quality};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::time::timeout;

fn sample_tag(path: &str, driver_id: &str, address: &str) -> Tag {
    Tag {
        path: path.to_string(),
        value: TagValue::new(ValueVariant::Int(0), Quality::Good),
        driver_id: driver_id.to_string(),
        driver_address: address.to_string(),
        poll_rate_ms: 1000,
        metadata: TagMetadata::default(),
    }
}

#[test]
fn test_duplicate_tag_registration() {
    let engine = TagEngine::new();
    let tag1 = sample_tag("Device/Tag1", "drv1", "addr1");
    let tag2 = sample_tag("Device/Tag1", "drv2", "addr2"); // Same path, different driver
    
    engine.register_tag(tag1.clone());
    engine.register_tag(tag2.clone()); // Should overwrite the first one
    
    let read = engine.read_tag("Device/Tag1").expect("tag should exist");
    // Should have the second tag's driver_id since it overwrote the first
    let details = engine.get_tag_details("Device/Tag1").expect("details should exist");
    assert_eq!(details.driver_id, "drv2");
}

#[test]
fn test_tag_value_types() {
    let engine = TagEngine::new();
    
    // Test different value types
    let bool_tag = Tag {
        path: "Test/Bool".to_string(),
        value: TagValue::new(ValueVariant::Bool(true), Quality::Good),
        driver_id: "test".to_string(),
        driver_address: "bool_addr".to_string(),
        poll_rate_ms: 1000,
        metadata: TagMetadata::default(),
    };
    
    let float_tag = Tag {
        path: "Test/Float".to_string(),
        value: TagValue::new(ValueVariant::Float(3.14159), Quality::Good),
        driver_id: "test".to_string(),
        driver_address: "float_addr".to_string(),
        poll_rate_ms: 1000,
        metadata: TagMetadata::default(),
    };
    
    let string_tag = Tag {
        path: "Test/String".to_string(),
        value: TagValue::new(ValueVariant::String("Hello World".to_string()), Quality::Good),
        driver_id: "test".to_string(),
        driver_address: "string_addr".to_string(),
        poll_rate_ms: 1000,
        metadata: TagMetadata::default(),
    };
    
    engine.register_tag(bool_tag.clone());
    engine.register_tag(float_tag.clone());
    engine.register_tag(string_tag.clone());
    
    // Verify all types are correctly stored and retrieved
    let bool_read = engine.read_tag("Test/Bool").unwrap();
    let float_read = engine.read_tag("Test/Float").unwrap();
    let string_read = engine.read_tag("Test/String").unwrap();
    
    assert_eq!(bool_read.value, ValueVariant::Bool(true));
    assert_eq!(float_read.value, ValueVariant::Float(3.14159));
    assert_eq!(string_read.value, ValueVariant::String("Hello World".to_string()));
}

#[test]
fn test_quality_levels() {
    let engine = TagEngine::new();
    
    let qualities = vec![
        Quality::Good,
        Quality::Uncertain,
        Quality::Bad,
        Quality::Initializing,
        Quality::CommFailure,
        Quality::ConfigError,
    ];
    
    for (i, quality) in qualities.iter().enumerate() {
        let tag = Tag {
            path: format!("Test/Quality{}", i),
            value: TagValue::new(ValueVariant::Int(i as i64), quality.clone()),
            driver_id: "test".to_string(),
            driver_address: format!("addr{}", i),
            poll_rate_ms: 1000,
            metadata: TagMetadata::default(),
        };
        
        engine.register_tag(tag);
        let read = engine.read_tag(&format!("Test/Quality{}", i)).unwrap();
        assert_eq!(read.quality, *quality);
    }
}

#[test]
fn test_concurrent_access() {
    let engine = Arc::new(TagEngine::new());
    let mut handles = vec![];
    
    // Register initial tags
    for i in 0..10 {
        let tag = sample_tag(&format!("Concurrent/Tag{}", i), "test", &format!("addr{}", i));
        engine.register_tag(tag);
    }
    
    // Spawn multiple threads that read and write concurrently
    for i in 0..5 {
        let engine_clone = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let tag_path = format!("Concurrent/Tag{}", j % 10);
                let new_value = TagValue::new(ValueVariant::Int((i * 100 + j) as i64), Quality::Good);
                
                // Try to update the tag value
                engine_clone.update_tag_value(&tag_path, new_value);
                
                // Try to read the tag
                if let Some(_) = engine_clone.read_tag(&tag_path) {
                    // Successfully read
                }
                
                // Small delay to increase chance of concurrent access
                thread::sleep(Duration::from_millis(1));
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify all tags still exist and are readable
    for i in 0..10 {
        let tag_path = format!("Concurrent/Tag{}", i);
        assert!(engine.read_tag(&tag_path).is_some());
    }
}

#[test]
fn test_large_number_of_tags() {
    let engine = TagEngine::new();
    let tag_count = 10000000;
    
    // Register a large number of tags
    for i in 0..tag_count {
        let tag = sample_tag(&format!("Load/Tag{:05}", i), "load_test", &format!("addr{}", i));
        engine.register_tag(tag);
    }
    
    // Verify we can read all tags
    let all_paths = engine.get_all_tag_paths();
    assert_eq!(all_paths.len(), tag_count);
    
    // Test random access
    for i in (0..tag_count).step_by(1000) {
        let tag_path = format!("Load/Tag{:05}", i);
        assert!(engine.read_tag(&tag_path).is_some());
    }
}

#[tokio::test]
async fn test_async_operations() {
    let engine = TagEngine::new();
    
    // Register some tags
    for i in 0..5 {
        let tag = sample_tag(&format!("Async/Tag{}", i), "async_test", &format!("addr{}", i));
        engine.register_tag(tag);
    }
    
    // Test async get_all_tags with timeout
    let result = timeout(Duration::from_secs(5), engine.get_all_tags()).await;
    assert!(result.is_ok());
    
    let all_tags = result.unwrap();
    assert_eq!(all_tags.len(), 5);
}

#[test]
fn test_tag_metadata() {
    let engine = TagEngine::new();
    
    let metadata = TagMetadata {
        description: Some("Temperature sensor reading".to_string()),
        eng_unit: Some("°C".to_string()),
        eng_low: Some(-40.0),
        eng_high: Some(120.0),
        writable: false,
    };
    
    let tag = Tag {
        path: "Plant/Temperature".to_string(),
        value: TagValue::new(ValueVariant::Float(25.5), Quality::Good),
        driver_id: "modbus1".to_string(),
        driver_address: "40001".to_string(),
        poll_rate_ms: 2000,
        metadata,
    };
    
    engine.register_tag(tag);
    
    let details = engine.get_tag_details("Plant/Temperature").expect("tag should exist");
    assert_eq!(details.metadata.description, Some("Temperature sensor reading".to_string()));
    assert_eq!(details.metadata.eng_unit, Some("°C".to_string()));
    assert_eq!(details.metadata.eng_low, Some(-40.0));
    assert_eq!(details.metadata.eng_high, Some(120.0));
    assert_eq!(details.metadata.writable, false);
}

#[test]
fn test_invalid_operations() {
    let engine = TagEngine::new();
    
    // Try to read a non-existent tag
    assert!(engine.read_tag("NonExistent/Tag").is_none());
    
    // Try to get details for a non-existent tag
    assert!(engine.get_tag_details("NonExistent/Tag").is_none());
    
    // Try to update a non-existent tag
    let new_value = TagValue::new(ValueVariant::Int(42), Quality::Good);
    assert!(!engine.update_tag_value("NonExistent/Tag", new_value));
    
    // Try to find path by non-existent driver/address combination
    assert!(engine.find_path_by_address("NonExistent", "addr").is_none());
}

#[test]
fn test_timestamp_functionality() {
    let engine = TagEngine::new();
    let tag = sample_tag("Time/Test", "driver", "addr");
    engine.register_tag(tag);
    
    let initial_read = engine.read_tag("Time/Test").unwrap();
    let initial_timestamp = initial_read.timestamp;
    
    // Wait a bit and update the tag
    thread::sleep(Duration::from_millis(10));
    let new_value = TagValue::new(ValueVariant::Int(100), Quality::Good);
    engine.update_tag_value("Time/Test", new_value);
    
    let updated_read = engine.read_tag("Time/Test").unwrap();
    let updated_timestamp = updated_read.timestamp;
    
    // Timestamp should have been updated
    assert!(updated_timestamp > initial_timestamp);
    assert_eq!(updated_read.value, ValueVariant::Int(100));
}
