use crate::drivers::traits::{DeviceDriver, DriverConfig, DriverResult, TagRequest};
use crate::tags::structures::{Quality, TagValue, ValueVariant};
use async_trait::async_trait;
use opcua::client::prelude::*;
use opcua::sync::RwLock;
use opcua::types::{NodeId, UAString};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

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
        NodeId::from_str(node_id_str)
            .map_err(|e| format!("Invalid NodeId '{}': {:?}", node_id_str, e).into())
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

    async fn connect(&self) -> DriverResult<()> {
        if self.client.lock().unwrap().is_some() {
            return Ok(());
        }

        let cfg = self.config.clone();
        let mut builder = ClientBuilder::new()
            .application_name(
                cfg
                    .application_name
                    .as_deref()
                    .unwrap_or("ForgeIO OPC UA Client"),
            )
            .application_uri(
                cfg
                    .application_uri
                    .as_deref()
                    .unwrap_or("urn:forgeio:client"),
            )
            .session_name(
                cfg
                    .session_name
                    .as_deref()
                    .unwrap_or("ForgeIOSession"),
            )
            .trust_server_certs(true)
            .create_sample_keypair(true);

        let size = cfg.max_message_size.unwrap_or(0);
        let chunks = cfg.max_chunk_count.unwrap_or(0);
        builder = builder.max_message_size(size).max_chunk_count(chunks);

        let address = cfg.address.clone();
        let (client, session) = tokio::task::block_in_place(|| {
            let mut client = builder.client().ok_or("failed to build client")?;
            let endpoint: EndpointDescription = (
                address.as_str(),
                "None",
                MessageSecurityMode::None,
                UserTokenPolicy::anonymous(),
            )
                .into();
            let session = client
                .connect_to_endpoint(endpoint, IdentityToken::Anonymous)
                .map_err(|e| format!("failed to connect: {:?}", e))?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((client, session))
        })?;

        info!("OPC UA driver connected to {}", self.config.address);

        *self.client.lock().unwrap() = Some(client);
        *self.session.lock().unwrap() = Some(session);
        Ok(())
    }

    async fn disconnect(&self) -> DriverResult<()> {
        let session = { self.session.lock().unwrap().take() };
        if let Some(session) = session {
            tokio::task::spawn_blocking(move || {
                session.read().disconnect();
            })
            .await
            .map_err(|e| format!("disconnect join error: {e}"))?;
        }
        *self.client.lock().unwrap() = None;
        Ok(())
    }

    async fn check_status(&self) -> DriverResult<()> {
        if let Some(session) = self.session.lock().unwrap().as_ref() {
            let session = session.read();
            if session.is_connected() {
                return Ok(());
            }
        }
        Err("Disconnected".into())
    }

    async fn read_tags(&self, tags: &[TagRequest]) -> DriverResult<HashMap<String, TagValue>> {
        let session_arc = {
            let guard = self.session.lock().unwrap();
            guard.clone().ok_or("not connected")?
        };
        let session = session_arc.write();

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

        info!(
            "OPC UA read {} values from {}",
            data_values.len(),
            self.config.address
        );

        for (req, dv) in tags.iter().zip(data_values.iter()) {
            match dv.status {
                Some(status) if status.is_good() => {
                    if let Some(val) = &dv.value {
                        info!("Read {} = {:?}", req.address, val);
                    } else {
                        info!("Read {} with empty value", req.address);
                    }
                }
                Some(status) => {
                    warn!("Bad status {:?} for node {}", status, req.address);
                }
                None => {
                    warn!("No status for node {}", req.address);
                }
            }
        }

        let mut result = HashMap::new();
        for (req, dv) in tags.iter().zip(data_values.iter()) {
            result.insert(req.address.clone(), Self::data_value_to_tag_value(dv));
        }
        Ok(result)
    }

    async fn write_tags(
        &self,
        _tags: HashMap<String, TagValue>,
    ) -> DriverResult<HashMap<String, TagValue>> {
        Ok(HashMap::new())
    }
}
