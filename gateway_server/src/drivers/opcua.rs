use crate::drivers::traits::{
    DeviceDriver, DriverConfig, DriverResult, TagRequest, TagValue, ValueVariant,
};
use async_trait::async_trait;
use opcua::{
    client::Client,
    types::{NodeId, ReadValueId, DataValue, Variant},
};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use tokio::sync::Mutex; // Using Mutex for interior mutability of the client

#[derive(Debug)] // Client is not Clone, so we derive Debug manually
pub struct OpcUaDriver {
    config: DriverConfig,
    // OPC UA Client needs to be mutable for operations, wrap in Mutex
    client: Mutex<Option<Client>>, 
}

impl OpcUaDriver {
    pub fn new(config: DriverConfig) -> Self {
        OpcUaDriver {
            config,
            client: Mutex::new(None),
        }
    }

    // Helper to parse NodeId strings (e.g., "ns=2;s=MyTag")
    fn parse_node_id(node_id_str: &str) -> Result<NodeId, Box<dyn Error + Send + Sync>> {
        // Basic parsing, a real implementation might need more robust error handling
        // or support for different NodeId types (numeric, guid, bytestring)
        let parts: Vec<&str> = node_id_str.split(';').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid NodeId format: {}", node_id_str).into());
        }
        let ns_part = parts[0];
        let identifier_part = parts[1];

        let ns = ns_part.trim_start_matches("ns=").parse::<u16>()?;

        if identifier_part.starts_with("s=") {
            Ok(NodeId::new_string(ns, identifier_part.trim_start_matches("s=").to_string()))
        } else if identifier_part.starts_with("i=") {
             Ok(NodeId::new_numeric(ns, identifier_part.trim_start_matches("i=").parse::<u32>()?))
        } else {
            Err(format!("Unsupported NodeId identifier format: {}", identifier_part).into())
        }
    }

    // Helper to convert OPC UA DataValue to our TagValue
    fn data_value_to_tag_value(dv: &DataValue) -> TagValue {
        let quality = match dv.status_code().is_good() { // TODO: Map more qualities
            true => crate::tags::structures::Quality::Good,
            false => crate::tags::structures::Quality::Bad,
        };
        let timestamp = dv.source_timestamp().map_or_else(
            || dv.server_timestamp().map_or(0, |dt| dt.timestamp_millis() as u64),
            |dt| dt.timestamp_millis() as u64
        );

        let value_variant = match dv.value() {
            Some(variant) => match variant {
                Variant::Boolean(b) => ValueVariant::Bool(*b),
                Variant::SByte(i) => ValueVariant::Int(*i as i64),
                Variant::Byte(u) => ValueVariant::UInt(*u as u64),
                Variant::Int16(i) => ValueVariant::Int(*i as i64),
                Variant::UInt16(u) => ValueVariant::UInt(*u as u64),
                Variant::Int32(i) => ValueVariant::Int(*i as i64),
                Variant::UInt32(u) => ValueVariant::UInt(*u as u64),
                Variant::Int64(i) => ValueVariant::Int(*i),
                Variant::UInt64(u) => ValueVariant::UInt(*u),
                Variant::Float(f) => ValueVariant::Float(*f as f64),
                Variant::Double(d) => ValueVariant::Float(*d),
                Variant::String(s) => ValueVariant::String(s.to_string()),
                // TODO: Handle more types (DateTime, arrays, etc.)
                _ => ValueVariant::Null, // Unsupported type for now
            },
            None => ValueVariant::Null,
        };

        TagValue {
            value: value_variant,
            quality,
            timestamp,
        }
    }
}

#[async_trait]
impl DeviceDriver for OpcUaDriver {
    fn config(&self) -> &DriverConfig {
        &self.config
    }

    async fn connect(&mut self) -> DriverResult<()> {
        // Ensure client is mutable through the Mutex guard
        let mut client_guard = self.client.lock().await;

        if client_guard.is_some() {
            println!("OPC UA Driver [{}]: Already connected or connecting.", self.config.id);
            return Ok(()); // Or perhaps check status?
        }

        println!("OPC UA Driver [{}]: Connecting to {}...", self.config.id, self.config.address);
        let endpoint_url = self.config.address.clone();

        // Create client config
        // TODO: Make security (policies, user identity) configurable
        let client = Client::new(&endpoint_url, None, None)?;

        // Connect (this is synchronous in the current opcua lib version used here)
        // The actual async connect/session activation is handled internally by the lib
        // when making calls like read/write.
        // We store the client instance to reuse the session.
        *client_guard = Some(client);

        println!("OPC UA Driver [{}]: Connection established (session pending activation on first call)", self.config.id);
        Ok(())
    }

    async fn disconnect(&mut self) -> DriverResult<()> {
        let mut client_guard = self.client.lock().await;
        if let Some(client) = client_guard.take() { // take() removes the value
            println!("OPC UA Driver [{}]: Disconnecting...", self.config.id);
            // client.disconnect() // The opcua crate handles session closing on drop/implicitly
             println!("OPC UA Driver [{}]: Disconnected.", self.config.id);
             Ok(())
        } else {
            println!("OPC UA Driver [{}]: Already disconnected.", self.config.id);
            Ok(())
        }
    }

    async fn check_status(&mut self) -> DriverResult<()> {
        let mut client_guard = self.client.lock().await;
        match client_guard.as_mut() {
            Some(client) => {
                // A simple way to check is to read a known node (e.g., ServerStatus)
                let node_id = NodeId::new_numeric(0, 2256); // ServerStatus NodeId
                let read_req = ReadValueId {
                    node_id,
                    attribute_id: 13, // Value attribute
                    index_range: None,
                    data_encoding: None,
                };
                match client.read(&[read_req], 0.0).await {
                    Ok(results) if !results.is_empty() && results[0].status_code().is_good() => Ok(()),
                    Ok(_) => Err("Failed to read server status or bad status code".into()),
                    Err(e) => Err(format!("OPC UA status check failed: {}", e).into()),
                }
            }
            None => Err("OPC UA client not connected".into()),
        }
    }

    async fn read_tags(&mut self, tags: &[TagRequest]) -> DriverResult<HashMap<String, TagValue>> {
        let mut client_guard = self.client.lock().await;
        let client = client_guard.as_mut().ok_or("OPC UA client not connected")?;

        let mut read_requests = Vec::with_capacity(tags.len());
        let mut node_id_map = HashMap::new(); // Map NodeId back to original string address

        for req in tags {
            let node_id = Self::parse_node_id(&req.address)?;
            node_id_map.insert(node_id.clone(), req.address.clone());
            read_requests.push(ReadValueId {
                node_id,
                attribute_id: 13, // Value attribute
                index_range: None,
                data_encoding: None,
            });
        }

        // Perform the read operation
        let results = client.read(&read_requests, 0.0).await?;

        let mut tag_values = HashMap::with_capacity(results.len());
        for (i, data_value) in results.iter().enumerate() {
            // Find the original NodeId string address using the index
            if let Some(node_id) = read_requests.get(i).map(|r| &r.node_id) {
                if let Some(original_address) = node_id_map.get(node_id) {
                     tag_values.insert(original_address.clone(), Self::data_value_to_tag_value(data_value));
                }
            }
        }

        Ok(tag_values)
    }

    async fn write_tags(&mut self, _tags: HashMap<String, TagValue>) -> DriverResult<HashMap<String, TagValue>> {
        // TODO: Implement OPC UA Write operation
        // 1. Lock the client
        // 2. Convert TagValue back to opcua::Variant + NodeId
        // 3. Create WriteValue structs
        // 4. Call client.write(...)
        // 5. Map results back to HashMap<String, TagValue> indicating success/failure
        println!("OPC UA Driver [{}]: Write functionality not yet implemented.", self.config.id);
        Err("Write not implemented".into())
    }
}
