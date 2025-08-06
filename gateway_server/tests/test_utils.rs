use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, DriverConfig, TagRequest};
use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Tag, TagValue, ValueVariant, TagMetadata, Quality};
use gateway_server::config::settings::{Settings, TagConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Test fixture for creating a populated TagEngine with sample data
pub struct TagEngineFixture {
    pub engine: Arc<TagEngine>,
    pub tag_count: usize,
}

impl TagEngineFixture {
    pub fn new(tag_count: usize) -> Self {
        let engine = Arc::new(TagEngine::new());
        
        for i in 0..tag_count {
            let tag = Self::create_test_tag(i);
            engine.register_tag(tag);
        }
        
        Self {
            engine,
            tag_count,
        }
    }
    
    pub fn create_test_tag(index: usize) -> Tag {
        let value_type = match index % 4 {
            0 => ValueVariant::Int(index as i64),
            1 => ValueVariant::Float(index as f64 * 1.5),
            2 => ValueVariant::Bool(index % 2 == 0),
            3 => ValueVariant::String(format!("Value_{}", index)),
            _ => ValueVariant::Int(0),
        };
        
        let quality = match index % 6 {
            0 => Quality::Good,
            1 => Quality::Uncertain,
            2 => Quality::Bad,
            3 => Quality::Initializing,
            4 => Quality::CommFailure,
            5 => Quality::ConfigError,
            _ => Quality::Good,
        };
        
        Tag {
            path: format!("TestDevice{}/Tag{:04}", index / 100, index % 100),
            value: TagValue::new(value_type, quality),
            driver_id: format!("driver_{}", index % 5),
            driver_address: format!("addr_{}", index),
            poll_rate_ms: 1000 + (index as u64 % 5) * 500, // Vary poll rates
            metadata: TagMetadata {
                description: Some(format!("Test tag number {}", index)),
                eng_unit: match index % 3 {
                    0 => Some("°C".to_string()),
                    1 => Some("bar".to_string()),
                    2 => Some("rpm".to_string()),
                    _ => None,
                },
                eng_low: Some((index as f64) * -10.0),
                eng_high: Some((index as f64) * 10.0),
                writable: index % 3 == 0,
            },
        }
    }
    
    pub fn get_sample_paths(&self, count: usize) -> Vec<String> {
        (0..count.min(self.tag_count))
            .map(|i| format!("TestDevice{}/Tag{:04}", i / 100, i % 100))
            .collect()
    }
    
    pub fn get_tags_by_driver(&self, driver_id: &str) -> Vec<String> {
        self.engine
            .get_all_tag_paths()
            .into_iter()
            .filter(|path| {
                if let Some(details) = self.engine.get_tag_details(path) {
                    details.driver_id == driver_id
                } else {
                    false
                }
            })
            .collect()
    }
}

/// Test fixture for creating OPC UA driver configurations
pub struct OpcUaDriverFixture;

impl OpcUaDriverFixture {
    pub fn create_config(id: &str, address: &str) -> DriverConfig {
        DriverConfig {
            id: id.to_string(),
            name: format!("Test Driver {}", id),
            address: address.to_string(),
            scan_rate_ms: 1000,
            application_name: Some(format!("TestApp_{}", id)),
            application_uri: Some(format!("urn:test:app:{}", id)),
            session_name: Some(format!("TestSession_{}", id)),
            max_message_size: Some(16777216),
            max_chunk_count: Some(1024),
            connect_retry_attempts: Some(3),
            connect_retry_delay_ms: Some(500),
            connect_retry_backoff: Some(2.0),
            connect_timeout_ms: Some(3000),
        }
    }
    
    pub fn create_fast_fail_config(id: &str, address: &str) -> DriverConfig {
        DriverConfig {
            id: id.to_string(),
            name: format!("Fast Fail Driver {}", id),
            address: address.to_string(),
            scan_rate_ms: 1000,
            application_name: Some(format!("TestApp_{}", id)),
            application_uri: None,
            session_name: Some(format!("TestSession_{}", id)),
            max_message_size: None,
            max_chunk_count: None,
            connect_retry_attempts: Some(1), // Only one attempt
            connect_retry_delay_ms: Some(100), // Short delay
            connect_retry_backoff: Some(1.0), // No backoff
            connect_timeout_ms: Some(500), // Short timeout
        }
    }
    
    pub async fn create_driver_with_config(config: DriverConfig) -> Result<OpcUaDriver, Box<dyn std::error::Error>> {
        Ok(OpcUaDriver::new(config)?)
    }
    
    pub fn create_tag_requests(count: usize) -> Vec<TagRequest> {
        (0..count)
            .map(|i| TagRequest {
                address: format!("ns=2;s=TestVar{}", i),
            })
            .collect()
    }
}

/// Test fixture for creating comprehensive system configurations
pub struct SystemConfigFixture;

impl SystemConfigFixture {
    pub fn create_multi_driver_config() -> Settings {
        let devices = vec![
            DriverConfig {
                id: "opcua1".to_string(),
                name: "Primary OPC UA Server".to_string(),
                address: "opc.tcp://127.0.0.1:4840/".to_string(),
                scan_rate_ms: 1000,
                application_name: Some("ForgeIO Client 1".to_string()),
                application_uri: Some("urn:forgeio:client1".to_string()),
                session_name: Some("ForgeIOSession1".to_string()),
                max_message_size: Some(16777216),
                max_chunk_count: Some(1024),
                connect_retry_attempts: Some(5),
                connect_retry_delay_ms: Some(1000),
                connect_retry_backoff: Some(2.0),
                connect_timeout_ms: Some(5000),
            },
            DriverConfig {
                id: "opcua2".to_string(),
                name: "Secondary OPC UA Server".to_string(),
                address: "opc.tcp://127.0.0.1:4841/".to_string(),
                scan_rate_ms: 2000,
                application_name: Some("ForgeIO Client 2".to_string()),
                application_uri: Some("urn:forgeio:client2".to_string()),
                session_name: Some("ForgeIOSession2".to_string()),
                max_message_size: Some(8388608),
                max_chunk_count: Some(512),
                connect_retry_attempts: Some(3),
                connect_retry_delay_ms: Some(2000),
                connect_retry_backoff: Some(1.5),
                connect_timeout_ms: Some(3000),
            },
        ];
        
        let tags = vec![
            TagConfig {
                path: "Plant1/Temperature".to_string(),
                driver_id: "opcua1".to_string(),
                address: "ns=2;s=Temperature".to_string(),
                poll_rate_ms: 1000,
            },
            TagConfig {
                path: "Plant1/Pressure".to_string(),
                driver_id: "opcua1".to_string(),
                address: "ns=2;s=Pressure".to_string(),
                poll_rate_ms: 1000,
            },
            TagConfig {
                path: "Plant2/Flow".to_string(),
                driver_id: "opcua2".to_string(),
                address: "ns=2;s=Flow".to_string(),
                poll_rate_ms: 2000,
            },
            TagConfig {
                path: "Plant2/Level".to_string(),
                driver_id: "opcua2".to_string(),
                address: "ns=2;s=Level".to_string(),
                poll_rate_ms: 2000,
            },
        ];
        
        Settings { devices, tags }
    }
    
    pub fn create_stress_test_config(device_count: usize, tags_per_device: usize) -> Settings {
        let mut devices = Vec::new();
        let mut tags = Vec::new();
        
        for device_idx in 0..device_count {
            let device = DriverConfig {
                id: format!("device_{}", device_idx),
                name: format!("Stress Test Device {}", device_idx),
                address: format!("opc.tcp://127.0.0.1:{}/", 4840 + device_idx),
                scan_rate_ms: 1000,
                application_name: Some(format!("StressApp_{}", device_idx)),
                application_uri: Some(format!("urn:stress:app:{}", device_idx)),
                session_name: Some(format!("StressSession_{}", device_idx)),
                max_message_size: Some(16777216),
                max_chunk_count: Some(1024),
                connect_retry_attempts: Some(2),
                connect_retry_delay_ms: Some(500),
                connect_retry_backoff: Some(1.5),
                connect_timeout_ms: Some(2000),
            };
            devices.push(device);
            
            for tag_idx in 0..tags_per_device {
                let tag = TagConfig {
                    path: format!("Device{}/Tag{:04}", device_idx, tag_idx),
                    driver_id: format!("device_{}", device_idx),
                    address: format!("ns=2;s=Tag{}", tag_idx),
                    poll_rate_ms: 1000 + (tag_idx as u64 % 3) * 500,
                };
                tags.push(tag);
            }
        }
        
        Settings { devices, tags }
    }
}

/// Utilities for test timing and performance measurement
pub struct TestTimer {
    start: std::time::Instant,
    name: String,
}

impl TestTimer {
    pub fn new(name: &str) -> Self {
        println!("Starting timer: {}", name);
        Self {
            start: std::time::Instant::now(),
            name: name.to_string(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    pub fn checkpoint(&self, checkpoint_name: &str) {
        let elapsed = self.elapsed();
        println!("Timer '{}' - Checkpoint '{}': {:?}", self.name, checkpoint_name, elapsed);
    }
}

impl Drop for TestTimer {
    fn drop(&mut self) {
        let elapsed = self.elapsed();
        println!("Timer '{}' completed in: {:?}", self.name, elapsed);
    }
}

/// Utility functions for common test operations
pub mod test_ops {
    use super::*;
    
    pub async fn wait_for_condition<F>(mut condition: F, timeout: Duration, check_interval: Duration) -> bool
    where
        F: FnMut() -> bool,
    {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if condition() {
                return true;
            }
            sleep(check_interval).await;
        }
        
        false
    }
    
    pub fn generate_load_tags(count: usize, base_path: &str) -> Vec<Tag> {
        (0..count)
            .map(|i| Tag {
                path: format!("{}/LoadTag{:06}", base_path, i),
                value: TagValue::new(
                    ValueVariant::Float(i as f64 * 0.1),
                    if i % 10 == 0 { Quality::Bad } else { Quality::Good }
                ),
                driver_id: format!("load_driver_{}", i % 5),
                driver_address: format!("load_addr_{}", i),
                poll_rate_ms: 1000 + (i as u64 % 10) * 100,
                metadata: TagMetadata {
                    description: Some(format!("Load test tag {}", i)),
                    eng_unit: Some("test".to_string()),
                    eng_low: Some(0.0),
                    eng_high: Some(100.0),
                    writable: i % 4 == 0,
                },
            })
            .collect()
    }
    
    pub fn assert_performance_threshold(actual: f64, expected_min: f64, metric_name: &str) {
        println!("{}: {:.2} (minimum: {:.2})", metric_name, actual, expected_min);
        assert!(
            actual >= expected_min,
            "{} ({:.2}) below expected minimum ({:.2})",
            metric_name,
            actual,
            expected_min
        );
    }
    
    pub fn log_test_results(test_name: &str, results: HashMap<String, f64>) {
        println!("\n=== {} Results ===", test_name);
        for (metric, value) in results {
            println!("  {}: {:.2}", metric, value);
        }
        println!("===============================\n");
    }
}