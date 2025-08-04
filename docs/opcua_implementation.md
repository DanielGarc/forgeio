# OPC UA Implementation Guide

This document describes the OPC UA functionality implemented in the ForgeIO Gateway Server.

## Features Implemented

### ✅ Async OPC UA Operations
All OPC UA operations are implemented using Rust's async/await pattern with tokio, providing non-blocking I/O and efficient concurrent handling of multiple connections.

### ✅ Multiple OPC UA Client Configuration
The system supports configuring multiple OPC UA clients through the `config.toml` file. Each client can have independent settings:

```toml
[[devices]]
id = "opcua1"
name = "Primary OPC UA Server"
address = "opc.tcp://127.0.0.1:4840/"
application_name = "ForgeIO OPC UA Client 1"
session_name = "ForgeIOSession1"
connect_retry_attempts = 5
connect_retry_delay_ms = 500
connect_retry_backoff = 2.0
connect_timeout_ms = 3000

[[devices]]
id = "opcua2"
name = "Secondary OPC UA Server"
address = "opc.tcp://192.168.1.100:4840/"
# ... other settings
```

### ✅ Tag Discovery and Browsing

#### REST API Endpoints

1. **Discover OPC UA Drivers**
   ```
   GET /api/opcua/discover
   ```
   Returns list of configured OPC UA drivers and their connection status.

2. **Browse OPC UA Node Hierarchy**
   ```
   GET /api/opcua/browse/{driver_id}?node_id={node_id}
   ```
   Browse children of a specific OPC UA node.

3. **Auto-Discover Tags**
   ```
   GET /api/opcua/discover-tags/{driver_id}
   ```
   Automatically discover available data variables on an OPC UA server.

#### Example API Usage

```bash
# Discover all OPC UA drivers
curl -u admin:admin http://127.0.0.1:3000/api/opcua/discover

# Browse the root Objects folder
curl -u admin:admin "http://127.0.0.1:3000/api/opcua/browse/opcua1?node_id=ns=0;i=85"

# Auto-discover all tags on a server
curl -u admin:admin http://127.0.0.1:3000/api/opcua/discover-tags/opcua1
```

### ✅ Admin Dashboard Integration

The admin dashboard includes a new **OPC UA Browser** page with:

- **Driver Selection**: Choose from available OPC UA drivers
- **Connection Status**: Visual indication of driver connection status
- **Interactive Node Browsing**: Navigate through the OPC UA node hierarchy
- **Tag Discovery**: Automatically find available tags on servers

Access via: `http://127.0.0.1:3000/opcua-browser`

## Configuration Options

### Device Configuration

Each OPC UA device supports the following configuration parameters:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `id` | Unique identifier for the device | Required |
| `name` | Human-readable name | Required |
| `address` | OPC UA endpoint URL | Required |
| `scan_rate_ms` | Default polling rate for tags | Required |
| `application_name` | OPC UA application name | "ForgeIO OPC UA Client" |
| `session_name` | OPC UA session name | "ForgeIOSession" |
| `application_uri` | Application URI | "urn:forgeio:client" |
| `max_message_size` | Maximum message size | 16777216 |
| `max_chunk_count` | Maximum chunk count | 1024 |
| `connect_retry_attempts` | Number of connection retries | 5 |
| `connect_retry_delay_ms` | Initial retry delay | 500 |
| `connect_retry_backoff` | Retry delay multiplier | 2.0 |
| `connect_timeout_ms` | Connection timeout | 3000 |

### Tag Configuration

Each tag associated with an OPC UA device:

```toml
[[tags]]
path = "Plant1/Temperature"        # Unique tag path in ForgeIO
driver_id = "opcua1"               # Must match a device ID
address = "ns=2;s=Temperature"     # OPC UA NodeId
poll_rate_ms = 1000               # Tag-specific polling rate
```

## Architecture

### Driver Implementation

The `OpcUaDriver` implements the `DeviceDriver` trait and provides:

- **Connection Management**: Automatic connection, reconnection, and connection health monitoring
- **Tag Reading/Writing**: Efficient batch operations for tag I/O
- **Node Browsing**: Navigate OPC UA address space
- **Tag Discovery**: Automatically find available data variables
- **Error Handling**: Comprehensive error handling with retry logic

### API Integration

The REST API layer provides HTTP endpoints that:

- Use HTTP Basic Authentication (admin/admin by default)
- Return JSON responses with proper error handling
- Support multiple concurrent requests
- Provide detailed status information

### Admin Dashboard

The React-based admin dashboard offers:

- **Real-time Updates**: Live display of tag values and connection status
- **Interactive Browsing**: Point-and-click navigation of OPC UA nodes
- **Responsive Design**: Works on desktop and mobile devices
- **Error Handling**: User-friendly error messages and status indicators

## Testing

### Dummy OPC UA Server

A test server is included for development and testing:

```bash
# Start the dummy server
cargo run --example dummy_opcua_server

# Start the gateway server
cargo run --bin gateway_server
```

The dummy server exposes test variables:
- `ns=2;s=Temperature`
- `ns=2;s=Pressure`
- `ns=2;s=Counter`

### Integration Tests

Run the test suite to verify OPC UA functionality:

```bash
cargo test
```

## Usage Examples

### Basic Setup

1. Configure your OPC UA server in `config.toml`
2. Start the gateway server: `cargo run --bin gateway_server`
3. Access the admin dashboard: `http://127.0.0.1:3000`
4. Navigate to "OPC UA Browser" to explore your server

### Multiple Servers

Configure multiple OPC UA servers by adding additional `[[devices]]` sections to `config.toml`. Each server operates independently with its own connection parameters and retry logic.

### Production Deployment

For production use:

1. Update authentication credentials
2. Configure proper TLS certificates
3. Set appropriate connection timeouts and retry parameters
4. Monitor connection status through the API endpoints

## Security Considerations

- Default authentication is HTTP Basic (admin/admin)
- OPC UA connections use the server's security policy
- Certificate management is handled automatically
- Update default credentials for production use

## Troubleshooting

### Connection Issues

Check the logs for connection errors:
```bash
RUST_LOG=info cargo run --bin gateway_server
```

Common issues:
- Incorrect OPC UA endpoint URL
- Network connectivity problems
- Server certificate issues
- Authentication/authorization problems

### Browse/Discovery Issues

- Ensure the OPC UA server allows browsing
- Check node IDs are correctly formatted
- Verify user permissions for browsing operations

For more detailed troubleshooting, enable debug logging:
```bash
RUST_LOG=debug cargo run --bin gateway_server
```