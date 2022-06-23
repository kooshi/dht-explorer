use super::*;

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
                formatter.write_str(&format!("expected 6 or 18 bytes"))
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
