use crate::{node::Node, u160::U160};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt::{Debug, Display}, net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr}};
pub const IPV4_DHT_NODE_BYTES_LEN: usize = 26;
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct NodeInfo {
    pub id:   U160,
    pub addr: SocketAddr,
}

impl NodeInfo {
    pub fn distance(&self, other: &Self) -> U160 {
        self.id.distance(other.id)
    }

    pub fn ip4_to_bytes(&self) -> [u8; IPV4_DHT_NODE_BYTES_LEN] {
        let mut bytes = [0_u8; IPV4_DHT_NODE_BYTES_LEN];
        bytes[..20].copy_from_slice(&self.id.to_be_bytes());
        if let IpAddr::V4(ip) = self.addr.ip() {
            bytes[20..24].copy_from_slice(&ip.octets());
        } else {
            panic!()
        }
        bytes[24..].copy_from_slice(&self.addr.port().to_be_bytes());
        bytes
    }

    pub fn ip4_from_bytes(bytes: &[u8; IPV4_DHT_NODE_BYTES_LEN]) -> Self {
        let mut idbytes = [0_u8; 20];
        idbytes.copy_from_slice(&bytes[..20]);
        let id = U160::from_be_bytes(&idbytes);
        let mut portbytes = [0_u8; 2];
        portbytes.copy_from_slice(&bytes[24..]);
        let addr = SocketAddr::new(
            Ipv4Addr::new(bytes[20], bytes[21], bytes[22], bytes[23]).into(),
            u16::from_be_bytes(portbytes),
        );
        Self { id, addr }
    }
}

impl Display for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}-{}]", self.id, self.addr)
    }
}
impl Debug for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}-{}]", self.id, self.addr)
    }
}

impl Serialize for NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut bytes = self.id.to_be_bytes().to_vec();
        bytes.append(
            &mut match self.addr.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(ip) => ip.octets().to_vec(),
            }
            .to_vec(),
        );
        bytes.append(&mut self.addr.port().to_be_bytes().to_vec());
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> Deserialize<'de> for NodeInfo {
    fn deserialize<D>(deserializer: D) -> Result<NodeInfo, D::Error>
    where D: Deserializer<'de> {
        struct Visitor {}
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = NodeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(" 26 or 38 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where E: serde::de::Error {
                let ip: IpAddr = match v.len() {
                    26 => Ipv4Addr::new(v[20], v[21], v[22], v[23]).into(),
                    38 => {
                        let mut bytes = [0_u8; 16];
                        bytes.copy_from_slice(&v[20..36]);
                        Ipv6Addr::from(bytes).into()
                    }
                    _ => return Err(serde::de::Error::invalid_length(v.len(), &self)),
                };
                let mut idbytes = [0_u8; 20];
                idbytes.copy_from_slice(&v[..20]);
                let id = U160::from_be_bytes(&idbytes);
                let mut portbytes = [0_u8; 2];
                portbytes.copy_from_slice(&v[v.len() - 2..]);
                let port = u16::from_be_bytes(portbytes);
                Ok(NodeInfo { id, addr: SocketAddr::new(ip, port) })
            }
        }
        deserializer.deserialize_bytes(Visitor {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;
    use std::{net::SocketAddr, str::FromStr};
    #[test]
    fn new() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let host = NodeInfo { id: U160::rand(), addr: socket };
        let copy = NodeInfo::ip4_from_bytes(&host.ip4_to_bytes());
        assert_eq!(host, copy);
    }

    #[test]
    fn serde() {
        let node = NodeInfo { id: U160::rand(), addr: SocketAddr::from_str("127.0.0.1:1337").unwrap() };
        let bytes = bt_bencode::to_vec(&node).unwrap();
        println!("{}", utils::safe_string_from_slice(&bytes));
        let copy = bt_bencode::from_slice(&bytes).unwrap();
        assert_eq!(node, copy);

        let node = NodeInfo {
            id:   U160::rand(),
            addr: SocketAddr::from_str("[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:1337").unwrap(),
        };
        let bytes = bt_bencode::to_vec(&node).unwrap();
        println!("{}", utils::safe_string_from_slice(&bytes));
        let copy = bt_bencode::from_slice(&bytes).unwrap();
        assert_eq!(node, copy);
    }
}
