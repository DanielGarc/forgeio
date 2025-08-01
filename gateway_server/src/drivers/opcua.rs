use crate::drivers::traits::{DeviceDriver, DriverConfig, DriverResult, TagRequest};
use crate::tags::structures::{Quality, TagValue, ValueVariant};
use async_trait::async_trait;
use opcua::client::prelude::*;
use opcua::sync::RwLock;
use opcua::types::{Identifier, NodeId, UAString};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct OpcUaDriver {
    config: DriverConfig,
    client: Mutex<Option<Client>>,
    session: Mutex<Option<Arc<RwLock<Session>>>>,
}

impl OpcUaDriver {
    pub fn new(config: DriverConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(OpcUaDriver {
            config,
            client: Mutex::new(None),
            session: Mutex::new(None),
        })
    }

    fn parse_node_id(
        node_id_str: &str,
    ) -> Result<NodeId, Box<dyn std::error::Error + Send + Sync>> {
        let parts: Vec<&str> = node_id_str.split(';').collect();
        if parts.len() != 2 {
            return Err("Invalid NodeId format".into());
        }
        let ns_part = parts[0].trim_start_matches("ns=").parse::<u16>()?;
        let identifier_part = parts[1];
        if identifier_part.starts_with("s=") {
            Ok(NodeId::new(
                ns_part,
                Identifier::String(opcua::types::UAString::from(
                    identifier_part.trim_start_matches("s=").to_string(),
                )),
            ))
        } else if identifier_part.starts_with("i=") {
            Ok(NodeId::new(
                ns_part,
                Identifier::Numeric(identifier_part.trim_start_matches("i=").parse::<u32>()?),
            ))
        } else {
            Err("Unsupported NodeId identifier format".into())
        }
    }

    fn data_value_to_tag_value(dv: &DataValue) -> TagValue {
        let quality = match dv.status {
            Some(status) => {
                if status.is_good() {
                    Quality::Good
                } else {
                    Quality::Bad
                }
            }
            None => Quality::Bad,
        };

        let value_variant = match &dv.value {
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
                Variant::LocalizedText(text) => ValueVariant::String(text.text.to_string()),
                _ => ValueVariant::Null,
            },
            None => ValueVariant::Null,
        };

        TagValue::new(value_variant, quality)
    }

    fn tag_value_to_variant(tv: &TagValue) -> Variant {
        match &tv.value {
            ValueVariant::Bool(b) => Variant::Boolean(*b),
            ValueVariant::Int(i) => Variant::Int32(*i as i32),
            ValueVariant::UInt(u) => Variant::UInt32(*u as u32),
            ValueVariant::Float(f) => Variant::Double(*f),
            ValueVariant::String(s) => Variant::String(UAString::from(s.clone())),
            _ => Variant::Empty,
        }
    }

    fn sync_method(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// impl std::fmt::Debug for OpcUaDriver {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("OpcUaDriver")
//          .field("config", &self.config)
//          .field("client", &"Mutex<Client>")
//          .finish()
//     }
// }

#[async_trait]
impl DeviceDriver for OpcUaDriver {
    fn config(&self) -> &DriverConfig {
        &self.config
    }

    fn connect(&mut self) -> DriverResult<()> {
        let mut client_guard = self.client.lock().unwrap();
        if client_guard.is_some() {
            return Ok(());
        }

        let mut client = ClientBuilder::new()
            .application_name("ForgeIO OPC UA Client")
            .application_uri("urn:forgeio:client")
            .trust_server_certs(true)
            .max_message_size(0)
            .max_chunk_count(0)
            .create_sample_keypair(true)
            .client()
            .ok_or("failed to build client")?;

        let endpoint: EndpointDescription = (
            self.config.address.as_str(),
            "None",
            MessageSecurityMode::None,
            UserTokenPolicy::anonymous(),
        )
            .into();

        let session = client
            .connect_to_endpoint(endpoint, IdentityToken::Anonymous)
            .map_err(|e| format!("failed to connect: {:?}", e))?;

        *client_guard = Some(client);
        let mut session_guard = self.session.lock().unwrap();
        *session_guard = Some(session);
        Ok(())
    }

    fn disconnect(&mut self) -> DriverResult<()> {
        if let Some(session) = self.session.lock().unwrap().take() {
            session.read().disconnect();
        }
        *self.client.lock().unwrap() = None;
        Ok(())
    }

    async fn check_status(&mut self) -> DriverResult<()> {
        if let Some(session) = self.session.lock().unwrap().as_ref() {
            let session = session.read();
            if session.is_connected() {
                return Ok(());
            }
        }
        Err("Disconnected".into())
    }

    async fn read_tags(&mut self, tags: &[TagRequest]) -> DriverResult<HashMap<String, TagValue>> {
        let session_arc = {
            let guard = self.session.lock().unwrap();
            guard.clone().ok_or("not connected")?
        };
        let mut session = session_arc.write();

        let mut read_ids = Vec::new();
        for t in tags {
            let node_id = Self::parse_node_id(&t.address)?;
            read_ids.push(ReadValueId {
                node_id,
                attribute_id: AttributeId::Value as u32,
                index_range: UAString::null(),
                data_encoding: QualifiedName::null(),
            });
        }

        let data_values = session
            .read(&read_ids, TimestampsToReturn::Both, 0.0)
            .map_err(|e| format!("read error: {:?}", e))?;

        let mut result = HashMap::new();
        for (req, dv) in tags.iter().zip(data_values.iter()) {
            result.insert(req.address.clone(), Self::data_value_to_tag_value(dv));
        }
        Ok(result)
    }

    async fn write_tags(
        &mut self,
        _tags: HashMap<String, TagValue>,
    ) -> DriverResult<HashMap<String, TagValue>> {
        Ok(HashMap::new())
    }
}
