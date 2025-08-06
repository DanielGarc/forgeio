# ForgeIO Testing Framework

This directory contains a comprehensive testing suite for the ForgeIO Gateway Server, designed to ensure robustness, performance, and reliability of the industrial IoT platform.

## Test Structure

### Unit Tests
- **`tag_engine.rs`** - Basic unit tests for the Tag Engine
- **`tag_engine_extended.rs`** - Extended unit tests including edge cases, concurrent access, and data type validation
- **`opcua_driver.rs`** - Basic OPC UA driver integration tests with dummy server
- **`opcua_driver_extended.rs`** - Extended OPC UA driver tests including error handling and failure scenarios

### Integration Tests
- **`api_integration.rs`** - REST API endpoint testing including authentication and error handling
- **`performance_tests.rs`** - Performance and stress testing for scalability validation

### Test Utilities
- **`test_utils.rs`** - Comprehensive test utilities, fixtures, and mock implementations

## Test Categories

### 1. Unit Tests
Focus on individual component behavior:
- Tag Engine operations (register, read, update, delete)
- Data type handling (Int, Float, Bool, String)
- Quality level management
- Metadata operations
- Thread safety and concurrent access

### 2. Integration Tests
Test component interactions:
- OPC UA driver connectivity
- Tag discovery and browsing
- API endpoint functionality
- Authentication and authorization
- Error propagation and handling

### 3. Performance Tests
Validate system performance characteristics:
- Tag registration performance (target: >10,000 tags/sec)
- Tag read performance (target: >100,000 reads/sec)
- Tag update performance (target: >50,000 updates/sec)
- Concurrent access performance
- Memory usage scaling
- Mixed operation throughput

### 4. Stress Tests
Test system behavior under extreme conditions:
- Large tag counts (50,000+ tags)
- High concurrency (multiple threads)
- Connection failures and recovery
- Resource exhaustion scenarios

### 5. Edge Case Tests
Handle unusual or error conditions:
- Invalid configurations
- Network failures
- Malformed data
- Resource limitations
- Concurrent modifications

## Running Tests

### All Tests
```bash
cargo test --workspace
```

### Specific Test Suites
```bash
# Basic unit tests
cargo test --test tag_engine
cargo test --test opcua_driver

# Extended tests
cargo test --test tag_engine_extended
cargo test --test opcua_driver_extended

# Integration tests
cargo test --test api_integration

# Performance tests (may take time)
cargo test --test performance_tests --release
```

### Performance Testing
Performance tests should be run in release mode for accurate results:
```bash
cargo test --test performance_tests --release -- --nocapture
```

## Test Results Summary

### Current Performance Metrics
- **Tag Registration**: ~50,000 tags/second
- **Tag Reading**: ~500,000 reads/second
- **Tag Updates**: ~200,000 updates/second
- **Concurrent Access**: Scales well with multiple threads
- **Memory Usage**: ~200-300 bytes per tag

### Known Issues
- **Tag Path Lookup**: Currently ~800 lookups/second (target: 10,000+)
  - Optimization needed in the driver/address indexing mechanism
  - Consider implementing hash-based lookup for better performance

### Test Coverage Areas

#### ✅ Well Covered
- Basic Tag Engine operations
- OPC UA driver error handling
- API endpoint functionality
- Data type validation
- Quality level handling
- Concurrent access safety

#### ⚠️ Needs Attention
- Tag path lookup performance optimization
- WebSocket API testing
- Configuration file validation
- Long-running stability tests
- Memory leak detection

#### 🔄 Future Enhancements
- End-to-end workflow testing
- Multi-protocol driver testing
- Historical data functionality
- Real OPC UA server integration tests
- Load testing with realistic industrial scenarios

## Test Utilities

### TagEngineFixture
Provides pre-populated TagEngine instances with various tag configurations for consistent testing.

### OpcUaDriverFixture
Creates OPC UA driver configurations for different test scenarios including fast-fail and standard configurations.

### SystemConfigFixture
Generates comprehensive system configurations for multi-driver and stress testing scenarios.

### TestTimer
Performance measurement utility for timing operations and identifying bottlenecks.

### Mock Implementations
- **MockDriver**: Simulates device driver behavior for testing without real hardware
- **Test Operations**: Common testing patterns and utilities

## Adding New Tests

### Guidelines
1. **Descriptive Names**: Use clear, descriptive test names that explain what is being tested
2. **Isolated Tests**: Each test should be independent and not rely on other tests
3. **Error Testing**: Include both success and failure scenarios
4. **Performance Metrics**: Document expected performance characteristics
5. **Resource Cleanup**: Ensure tests clean up resources properly

### Example Test Structure
```rust
#[tokio::test]
async fn test_descriptive_name() {
    let _timer = TestTimer::new("test_descriptive_name");
    
    // Arrange
    let fixture = TagEngineFixture::new(100);
    
    // Act
    let result = fixture.engine.some_operation().await;
    
    // Assert
    assert!(result.is_ok());
    
    // Performance assertion if applicable
    test_ops::assert_performance_threshold(
        actual_performance, 
        expected_minimum, 
        "operation_name"
    );
}
```

## Continuous Integration

The testing framework is designed to work with CI systems:
- Fast unit tests run on every commit
- Integration tests run on pull requests
- Performance tests run on release candidates
- Stress tests run on scheduled builds

## Debugging Failed Tests

### Common Issues
1. **Timing Issues**: Use `TestTimer` to identify slow operations
2. **Resource Conflicts**: Ensure tests don't conflict over ports or files
3. **Environment Dependencies**: Mock external dependencies where possible
4. **Performance Variations**: Account for system load in performance assertions

### Debug Commands
```bash
# Run with detailed output
cargo test --test test_name -- --nocapture

# Run single test with backtrace
RUST_BACKTRACE=1 cargo test specific_test_name

# Run in release mode for performance tests
cargo test --test performance_tests --release
```

## Contributing

When adding new functionality:
1. Add corresponding unit tests
2. Update integration tests if APIs change
3. Add performance tests for critical paths
4. Update this documentation
5. Ensure all tests pass in CI

The testing framework is a critical part of maintaining ForgeIO's reliability and performance in industrial environments.
