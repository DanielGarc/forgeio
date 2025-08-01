use crate::drivers::traits::{DeviceDriver, DriverConfig, DriverResult, TagRequest};
use crate::tags::structures::{Quality, TagValue, ValueVariant};
use async_trait::async_trait;
use opcua::client::prelude::*;
use opcua::types::{Identifier, NodeId, UAString};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct OpcUaDriver {
    config: DriverConfig,
    client: Mutex<Option<Client>>,
}

impl OpcUaDriver {
    pub fn new(config: DriverConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(OpcUaDriver {
            config,
            client: Mutex::new(None),
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
        Ok(())
    }

    fn disconnect(&mut self) -> DriverResult<()> {
        Ok(())
    }

    async fn check_status(&mut self) -> DriverResult<()> {
        Ok(())
    }

    async fn read_tags(&mut self, _tags: &[TagRequest]) -> DriverResult<HashMap<String, TagValue>> {
        Ok(HashMap::new())
    }

    async fn write_tags(
        &mut self,
        _tags: HashMap<String, TagValue>,
    ) -> DriverResult<HashMap<String, TagValue>> {
        Ok(HashMap::new())
    }
}
