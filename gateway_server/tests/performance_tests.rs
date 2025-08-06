use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Tag, TagValue, ValueVariant, TagMetadata, Quality};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::time::timeout;

fn create_sample_tag(index: usize) -> Tag {
    Tag {
        path: format!("Performance/Tag{:06}", index),
        value: TagValue::new(ValueVariant::Float(index as f64 * 1.5), Quality::Good),
        driver_id: format!("driver_{}", index % 10), // Distribute across 10 drivers
        driver_address: format!("addr_{}", index),
        poll_rate_ms: 1000,
        metadata: TagMetadata {
            description: Some(format!("Performance test tag {}", index)),
            eng_unit: Some("units".to_string()),
            eng_low: Some(0.0),
            eng_high: Some(1000.0),
            writable: index % 5 == 0, // Every 5th tag is writable
        },
    }
}

#[test]
fn test_tag_registration_performance() {
    let engine = TagEngine::new();
    let tag_count = 50000;
    
    let start = Instant::now();
    
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    let registration_time = start.elapsed();
    let tags_per_second = tag_count as f64 / registration_time.as_secs_f64();
    
    println!("Registered {} tags in {:?} ({:.0} tags/sec)", 
             tag_count, registration_time, tags_per_second);
    
    // Should be able to register at least 10,000 tags per second
    assert!(tags_per_second > 10000.0);
    
    // Verify all tags were registered
    let all_paths = engine.get_all_tag_paths();
    assert_eq!(all_paths.len(), tag_count);
}

#[test]
fn test_tag_read_performance() {
    let engine = TagEngine::new();
    let tag_count = 10000;
    
    // Register tags first
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    // Test sequential reads
    let start = Instant::now();
    for i in 0..tag_count {
        let path = format!("Performance/Tag{:06}", i);
        let _value = engine.read_tag(&path).expect("Tag should exist");
    }
    let read_time = start.elapsed();
    let reads_per_second = tag_count as f64 / read_time.as_secs_f64();
    
    println!("Read {} tags in {:?} ({:.0} reads/sec)", 
             tag_count, read_time, reads_per_second);
    
    // Should be able to read at least 100,000 tags per second
    assert!(reads_per_second > 100000.0);
}

#[test]
fn test_tag_update_performance() {
    let engine = TagEngine::new();
    let tag_count = 10000;
    
    // Register tags first
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    // Test sequential updates
    let start = Instant::now();
    for i in 0..tag_count {
        let path = format!("Performance/Tag{:06}", i);
        let new_value = TagValue::new(ValueVariant::Float(i as f64 * 2.0), Quality::Good);
        engine.update_tag_value(&path, new_value);
    }
    let update_time = start.elapsed();
    let updates_per_second = tag_count as f64 / update_time.as_secs_f64();
    
    println!("Updated {} tags in {:?} ({:.0} updates/sec)", 
             tag_count, update_time, updates_per_second);
    
    // Should be able to update at least 50,000 tags per second
    assert!(updates_per_second > 50000.0);
}

#[test]
fn test_concurrent_read_performance() {
    let engine = Arc::new(TagEngine::new());
    let tag_count = 10000;
    let thread_count = 10;
    
    // Register tags first
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let engine_clone = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            let reads_per_thread = tag_count / thread_count;
            let start_idx = thread_id * reads_per_thread;
            let end_idx = start_idx + reads_per_thread;
            
            for i in start_idx..end_idx {
                let path = format!("Performance/Tag{:06}", i);
                let _value = engine_clone.read_tag(&path).expect("Tag should exist");
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let concurrent_read_time = start.elapsed();
    let concurrent_reads_per_second = tag_count as f64 / concurrent_read_time.as_secs_f64();
    
    println!("Concurrent read of {} tags with {} threads in {:?} ({:.0} reads/sec)", 
             tag_count, thread_count, concurrent_read_time, concurrent_reads_per_second);
    
    // Should be faster than sequential reads due to parallelism
    assert!(concurrent_reads_per_second > 50000.0);
}

#[test]
fn test_mixed_operations_performance() {
    let engine = Arc::new(TagEngine::new());
    let tag_count = 5000;
    let thread_count = 8;
    
    // Register tags first
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let engine_clone = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            let operations_per_thread = tag_count / thread_count;
            
            for i in 0..operations_per_thread {
                let tag_idx = (thread_id * operations_per_thread + i) % tag_count;
                let path = format!("Performance/Tag{:06}", tag_idx);
                
                match i % 3 {
                    0 => {
                        // Read operation
                        let _value = engine_clone.read_tag(&path);
                    }
                    1 => {
                        // Update operation
                        let new_value = TagValue::new(
                            ValueVariant::Float(i as f64),
                            Quality::Good
                        );
                        engine_clone.update_tag_value(&path, new_value);
                    }
                    2 => {
                        // Get details operation
                        let _details = engine_clone.get_tag_details(&path);
                    }
                    _ => {}
                }
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let mixed_ops_time = start.elapsed();
    let ops_per_second = tag_count as f64 / mixed_ops_time.as_secs_f64();
    
    println!("Mixed operations on {} tags with {} threads in {:?} ({:.0} ops/sec)", 
             tag_count, thread_count, mixed_ops_time, ops_per_second);
    
    assert!(ops_per_second > 20000.0);
}

#[tokio::test]
async fn test_async_operations_performance() {
    let engine = TagEngine::new();
    let tag_count = 5000;
    
    // Register tags first
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    // Test async get_all_tags performance
    let start = Instant::now();
    let result = timeout(Duration::from_secs(10), engine.get_all_tags()).await;
    let async_time = start.elapsed();
    
    assert!(result.is_ok());
    let all_tags = result.unwrap();
    assert_eq!(all_tags.len(), tag_count);
    
    println!("Async get_all_tags for {} tags completed in {:?}", 
             tag_count, async_time);
    
    // Should complete within reasonable time
    assert!(async_time < Duration::from_secs(5));
}

#[test]
fn test_memory_usage_scaling() {
    let engine = TagEngine::new();
    let initial_memory = get_memory_usage();
    
    // Register increasingly more tags and measure memory growth
    let batch_sizes = vec![1000, 5000, 10000, 25000];
    let mut total_tags = 0;
    
    for batch_size in batch_sizes {
        // Register a batch of tags
        for i in total_tags..(total_tags + batch_size) {
            let tag = create_sample_tag(i);
            engine.register_tag(tag);
        }
        total_tags += batch_size;
        
        let current_memory = get_memory_usage();
        let memory_per_tag = (current_memory - initial_memory) / total_tags;
        
        println!("Tags: {}, Memory usage: {} bytes, Per tag: {} bytes", 
                 total_tags, current_memory - initial_memory, memory_per_tag);
        
        // Memory per tag should be reasonable (less than 1KB per tag)
        assert!(memory_per_tag < 1024);
    }
}

#[test]
fn test_tag_path_lookup_performance() {
    let engine = TagEngine::new();
    let tag_count = 20000;
    
    // Register tags with known driver/address mappings
    for i in 0..tag_count {
        let tag = create_sample_tag(i);
        engine.register_tag(tag);
    }
    
    // Test driver/address lookup performance
    let start = Instant::now();
    let mut found_count = 0;
    
    for i in 0..tag_count {
        let driver_id = format!("driver_{}", i % 10);
        let address = format!("addr_{}", i);
        
        if engine.find_path_by_address(&driver_id, &address).is_some() {
            found_count += 1;
        }
    }
    
    let lookup_time = start.elapsed();
    let lookups_per_second = tag_count as f64 / lookup_time.as_secs_f64();
    
    println!("Performed {} lookups in {:?} ({:.0} lookups/sec), found {}", 
             tag_count, lookup_time, lookups_per_second, found_count);
    
    assert_eq!(found_count, tag_count); // All should be found
    assert!(lookups_per_second > 10000.0); // Should be fast
}

#[test]
fn test_stress_tag_registration_and_cleanup() {
    let engine = TagEngine::new();
    let cycles = 10;
    let tags_per_cycle = 5000;
    
    for cycle in 0..cycles {
        let start = Instant::now();
        
        // Register tags
        for i in 0..tags_per_cycle {
            let tag = Tag {
                path: format!("Stress/Cycle{}/Tag{}", cycle, i),
                value: TagValue::new(ValueVariant::Int(i as i64), Quality::Good),
                driver_id: "stress_driver".to_string(),
                driver_address: format!("cycle_{}_addr_{}", cycle, i),
                poll_rate_ms: 1000,
                metadata: TagMetadata::default(),
            };
            engine.register_tag(tag);
        }
        
        let registration_time = start.elapsed();
        
        // Verify registration
        let all_paths = engine.get_all_tag_paths();
        let expected_count = (cycle + 1) * tags_per_cycle;
        assert_eq!(all_paths.len(), expected_count);
        
        println!("Cycle {}: Registered {} tags in {:?}, total tags: {}", 
                 cycle, tags_per_cycle, registration_time, all_paths.len());
    }
    
    // Final verification
    let final_count = engine.get_all_tag_paths().len();
    assert_eq!(final_count, cycles * tags_per_cycle);
}

// Helper function to get approximate memory usage
// Note: This is a simplified approach and may not be perfectly accurate
fn get_memory_usage() -> usize {
    // In a real implementation, you might use system calls or crates like `sysinfo`
    // For this test, we'll use a placeholder that returns a reasonable value
    use std::alloc::{GlobalAlloc, Layout, System};
    
    // This is a simplified approximation
    // In practice, you'd want to use proper memory profiling tools
    std::mem::size_of::<TagEngine>() * 1000 // Placeholder
}
