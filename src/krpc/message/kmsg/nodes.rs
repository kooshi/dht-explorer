use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactIPv4NodeInfo {
    pub dht_nodes: Vec<DhtNode>,
}

impl Serialize for CompactIPv4NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let bytes = self.dht_nodes.iter().map(|n| n.ip4_to_bytes()).collect::<Vec<[u8; 26]>>().concat();
        serializer.serialize_bytes(&bytes)
    }
}
impl<'de> Deserialize<'de> for CompactIPv4NodeInfo {
    fn deserialize<D>(deserializer: D) -> Result<CompactIPv4NodeInfo, D::Error>
    where D: Deserializer<'de> {
        struct CompactIPv4NodeInfoVisitor {}
        impl<'de> serde::de::Visitor<'de> for CompactIPv4NodeInfoVisitor {
            type Value = CompactIPv4NodeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!("expected n * {} bytes", IPV4_DHT_NODE_BYTES_LEN))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(CompactIPv4NodeInfo {
                    dht_nodes: v
                        .chunks_exact(IPV4_DHT_NODE_BYTES_LEN)
                        .map(|c| {
                            let mut sized = [0u8; IPV4_DHT_NODE_BYTES_LEN];
                            sized.copy_from_slice(c);
                            DhtNode::ip4_from_bytes(&sized)
                        })
                        .collect(),
                })
            }
        }
        deserializer.deserialize_bytes(CompactIPv4NodeInfoVisitor {})
    }
}
