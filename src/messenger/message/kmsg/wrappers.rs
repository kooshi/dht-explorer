use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactIPv4NodeInfo {
    pub dht_nodes: Vec<NodeInfo>,
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
                            NodeInfo::ip4_from_bytes(&sized)
                        })
                        .collect(),
                })
            }
        }
        deserializer.deserialize_bytes(CompactIPv4NodeInfoVisitor {})
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactInfoHashes {
    pub info_hashes: Vec<U160>,
}

impl Serialize for CompactInfoHashes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let bytes = self.info_hashes.iter().map(|n| n.to_be_bytes()).collect::<Vec<[u8; 20]>>().concat();
        serializer.serialize_bytes(&bytes)
    }
}
impl<'de> Deserialize<'de> for CompactInfoHashes {
    fn deserialize<D>(deserializer: D) -> Result<CompactInfoHashes, D::Error>
    where D: Deserializer<'de> {
        struct CompactInfoHashesVisitor {}
        impl<'de> serde::de::Visitor<'de> for CompactInfoHashesVisitor {
            type Value = CompactInfoHashes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!("expected n * {} bytes", 20))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(CompactInfoHashes {
                    info_hashes: v
                        .chunks_exact(20)
                        .map(|c| {
                            let mut sized = [0u8; 20];
                            sized.copy_from_slice(c);
                            U160::from_be_bytes(&sized)
                        })
                        .collect(),
                })
            }
        }
        deserializer.deserialize_bytes(CompactInfoHashesVisitor {})
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SocketAddrWrapper {
    pub socket_addr: Option<SocketAddr>,
}

impl Serialize for SocketAddrWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        if let Some(addr) = self.socket_addr {
            let mut bytes = match addr.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(ip) => ip.octets().to_vec(),
            };
            bytes.append(&mut addr.port().to_be_bytes().to_vec());
            serializer.serialize_bytes(&bytes)
        } else {
            serializer.serialize_bytes(&[])
        }
    }
}

impl<'de> Deserialize<'de> for SocketAddrWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        struct SocketAddrWrapperVisitor {}
        impl<'de> serde::de::Visitor<'de> for SocketAddrWrapperVisitor {
            type Value = SocketAddrWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expected 6 or 18 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                let ip: Option<IpAddr> = match v.len() {
                    6 => Some(Ipv4Addr::new(v[0], v[1], v[2], v[3]).into()),
                    18 => {
                        let mut bytes = [0_u8; 16];
                        bytes.copy_from_slice(&v[..16]);
                        Some(Ipv6Addr::from(bytes).into())
                    }
                    _ => None,
                };
                if let Some(ip) = ip {
                    let mut portbytes = [0_u8; 2];
                    portbytes.copy_from_slice(&v[v.len() - 2..]);
                    let port = u16::from_be_bytes(portbytes);
                    Ok(SocketAddrWrapper { socket_addr: Some(SocketAddr::new(ip, port)) })
                } else {
                    Ok(SocketAddrWrapper { socket_addr: None })
                }
            }
        }
        deserializer.deserialize_bytes(SocketAddrWrapperVisitor {})
    }
}
