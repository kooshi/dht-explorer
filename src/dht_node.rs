use crate::u160::U160;
use std::net::{self, SocketAddr, IpAddr, Ipv4Addr};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DhtNode {
    pub id: U160,
    pub addr: SocketAddr,
}
impl DhtNode {
    // pub fn new() -> Self {
    //     let (foo,addr) = net::UdpSocket::bind("127.0.0.1:3400").expect("couldn't bind to address").recv_from([0_u8;10].as_mut());
    //     DhtNode { id:U160::new() }
    // }
    pub fn distance(&self, other: &Self) -> U160 {
        self.id.distance(other.id)
    }

    pub fn ip4_to_bytes(&self) -> [u8;26] {
        let mut bytes = [0_u8;26];
        bytes[..20].copy_from_slice(&self.id.to_be_bytes());
        if let IpAddr::V4(ip) = self.addr.ip() {
            bytes[20..24].copy_from_slice( &ip.octets());
        } else {
            panic!()
        }
        bytes[24..].copy_from_slice(&self.addr.port().to_be_bytes());
        bytes
    }

    pub fn ip4_from_bytes(bytes:[u8;26]) -> Self {
        let mut idbytes = [0_u8;20];
        idbytes.copy_from_slice(&bytes[..20]);
        let id = U160::from_be_bytes(idbytes);
        let mut portbytes = [0_u8;2];
        portbytes.copy_from_slice(&bytes[24..]);
        let addr = SocketAddr::new(Ipv4Addr::new(bytes[20], bytes[21], bytes[22], bytes[23]).into(),u16::from_be_bytes(portbytes));
        Self { id, addr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::SocketAddrV4, str::FromStr};
    #[test]
    fn new() {
        let socket = std::net::SocketAddr::from(SocketAddrV4::from_str("127.0.0.1:1337").unwrap());
        let host = DhtNode { id: U160::new(), addr: socket };
        let copy = DhtNode::ip4_from_bytes(host.ip4_to_bytes());
        assert_eq!(host,copy);
    }
}
