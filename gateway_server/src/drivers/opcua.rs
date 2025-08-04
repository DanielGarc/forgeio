use crate::drivers::traits::{DeviceDriver, DriverConfig, DriverResult, TagRequest};
use crate::tags::structures::{Quality, TagValue, ValueVariant};
use async_trait::async_trait;
use opcua::client::{Client, ClientBuilder, IdentityToken, Session};
use opcua::types::{
    AttributeId, BrowseDescription, BrowseDirection, BrowseResultMask, DataValue,
    EndpointDescription, MessageSecurityMode, NodeId, QualifiedName, ReadValueId, ReferenceTypeId,
    TimestampsToReturn, UAString, UserTokenPolicy, Variant,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct OpcUaDriver {
    config: DriverConfig,
    client: Mutex<Option<Client>>,
    session: Mutex<Option<Arc<Session>>>,
    event_loop: Mutex<Option<tokio::task::JoinHandle<opcua::types::StatusCode>>>,
}

impl OpcUaDriver {
    pub fn new(config: DriverConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config,
            client: Mutex::new(None),
            session: Mutex::new(None),
            event_loop: Mutex::new(None),
        })
    }

    fn parse_node_id(
        node_id_str: &str,
    ) -> Result<NodeId, Box<dyn std::error::Error + Send + Sync>> {
        NodeId::from_str(node_id_str)
            .map_err(|e| format!("Invalid NodeId '{}': {e:?}", node_id_str).into())
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

    #[allow(dead_code)]
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

    pub async fn browse_node(&self, node_id_str: &str) -> DriverResult<Vec<String>> {
        let session = {
            let guard = self.session.lock().unwrap();
            guard.clone().ok_or("not connected")?
        };

        let node_id = Self::parse_node_id(node_id_str)?;
        let browse_desc = BrowseDescription {
            node_id,
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::HierarchicalReferences.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseResultMask::All as u32,
        };

        let results = session
            .browse(&[browse_desc], 0, None)
            .await
            .map_err(|e| format!("browse error: {e:?}"))?;

        let mut names = Vec::new();
        if let Some(res) = results.get(0) {
            if let Some(refs) = &res.references {
                for reference in refs {
                    names.push(reference.browse_name.name.to_string());
                }
            }
        }
        Ok(names)
    }
}

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
        let max_retries = cfg.connect_retry_attempts.unwrap_or(0);
        let mut delay = cfg.connect_retry_delay_ms.unwrap_or(0);
        let backoff = cfg.connect_retry_backoff.unwrap_or(2.0);
        let timeout_ms = cfg.connect_timeout_ms.unwrap_or(5_000);
        let mut attempt = 0;

        loop {
            let attempt_fut = async {
                let mut client = ClientBuilder::new()
                    .application_name(
                        cfg.application_name
                            .as_deref()
                            .unwrap_or("ForgeIO OPC UA Client"),
                    )
                    .application_uri(
                        cfg.application_uri
                            .as_deref()
                            .unwrap_or("urn:forgeio:client"),
                    )
                    .session_name(cfg.session_name.as_deref().unwrap_or("ForgeIOSession"))
                    .trust_server_certs(true)
                    .create_sample_keypair(true)
                    .max_message_size(cfg.max_message_size.unwrap_or(0))
                    .max_chunk_count(cfg.max_chunk_count.unwrap_or(0))
                    .client()
                    .map_err(|e| format!("failed to build client: {e:?}"))?;

                let endpoint: EndpointDescription = (
                    cfg.address.as_str(),
                    "None",
                    MessageSecurityMode::None,
                    UserTokenPolicy::anonymous(),
                )
                    .into();

                let (session, event_loop) = client
                    .connect_to_matching_endpoint(endpoint, IdentityToken::Anonymous)
                    .await
                    .map_err(|e| format!("failed to connect: {e:?}"))?;

                let mut handle = event_loop.spawn();
                tokio::select! {
                    status = &mut handle => {
                        Err(format!("event loop ended: {status:?}"))
                    }
                    _ = session.wait_for_connection() => {
                        Ok((client, session, handle))
                    }
                }
            };

            match tokio::time::timeout(Duration::from_millis(timeout_ms), attempt_fut).await {
                Ok(Ok((client, session, handle))) => {
                    *self.client.lock().unwrap() = Some(client);
                    *self.session.lock().unwrap() = Some(session);
                    *self.event_loop.lock().unwrap() = Some(handle);
                    info!("OPC UA driver connected to {}", self.config.address);
                    return Ok(());
                }
                Ok(Err(e)) if attempt < max_retries => {
                    warn!(
                        "OPC UA connection attempt {} failed: {}. Retrying in {} ms",
                        attempt + 1,
                        e,
                        delay
                    );
                }
                Ok(Err(e)) => return Err(e.into()),
                Err(_) if attempt < max_retries => {
                    warn!(
                        "OPC UA connection attempt {} timed out after {} ms. Retrying in {} ms",
                        attempt + 1,
                        timeout_ms,
                        delay
                    );
                }
                Err(_) => {
                    return Err(
                        format!("connection attempt timed out after {} ms", timeout_ms).into(),
                    )
                }
            }

            if delay > 0 {
                sleep(Duration::from_millis(delay)).await;
                delay = (delay as f64 * backoff) as u64;
            }
            attempt += 1;
        }
    }

    async fn disconnect(&self) -> DriverResult<()> {
        let session = { self.session.lock().unwrap().take() };
        if let Some(session) = session {
            session
                .disconnect()
                .await
                .map_err(|e| format!("disconnect error: {e:?}"))?;
        }
        let handle = { self.event_loop.lock().unwrap().take() };
        if let Some(handle) = handle {
            let _ = handle.await;
        }
        *self.client.lock().unwrap() = None;
        Ok(())
    }

    async fn check_status(&self) -> DriverResult<()> {
        if let Some(session) = self.session.lock().unwrap().as_ref() {
            if session.server_session_id() != NodeId::null() {
                return Ok(());
            }
        }
        Err("Disconnected".into())
    }

    async fn read_tags(&self, tags: &[TagRequest]) -> DriverResult<HashMap<String, TagValue>> {
        let session = {
            let guard = self.session.lock().unwrap();
            guard.clone().ok_or("not connected")?
        };

        let mut read_ids = Vec::new();
        for t in tags {
            let node_id = Self::parse_node_id(&t.address)?;
            read_ids.push(ReadValueId {
                node_id,
                attribute_id: AttributeId::Value as u32,
                index_range: Default::default(),
                data_encoding: QualifiedName::null(),
            });
        }

        let data_values = session
            .read(&read_ids, TimestampsToReturn::Both, 0.0)
            .await
            .map_err(|e| format!("read error: {e:?}"))?;

        info!(
            "OPC UA read {} values from {}",
            data_values.len(),
            self.config.address
        );

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
