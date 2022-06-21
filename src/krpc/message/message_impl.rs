use std::fmt::Display;

use super::kmsg::KMessage;
use super::*;
use kmsg;
use serde::de;
use serde::ser;
use serde::{Deserialize, Deserializer, Serialize};
impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_kmsg().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Message, D::Error>
    where
        D: Deserializer<'de>,
    {
        Message::from_kmsg(KMessage::deserialize(deserializer)?).map_err(|e|de::Error::custom(e))
    }
}

impl Message {
    pub fn receive(bytes:&[u8], from:SocketAddr) -> Result<Self, SimpleError> {
        Self::from_bytes(bytes).map(|mut s|{s.received_from_addr = Some(from);s})
    }

    pub fn from_bytes(bytes:&[u8]) -> Result<Self, SimpleError> {
        bt_bencode::from_slice(bytes).map_err(|e|SimpleError::with("error getting message from bytes", e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bt_bencode::to_vec(self).unwrap()
    }

    pub fn to_kmsg(&self) -> KMessage {
        let builder = KMessage::builder()
        .transaction_id(self.transaction_id.clone())
        .peer_ip(socket_addr_wrapper::SocketAddrWrapper { socket_addr: self.destination_addr})
        .read_only(self.read_only);
        match &self.kind {
            MessageKind::Query(q) => {
                let mut args = kmsg::MessageArgs::builder().id(self.sender_id).build();
                let builder = builder.message_type(kmsg::Y_QUERY);
                let builder = match q {
                    QueryMethod::Ping => builder.query_method(Q_PING),
                    QueryMethod::FindNode(target) => { 
                        args.target = Some(*target);
                        builder.query_method(Q_FIND_NODE)
                    },
                    QueryMethod::GetPeers(info_hash) => {
                        args.info_hash = Some(*info_hash);
                        builder.query_method(Q_GET_PEERS)
                    },
                    QueryMethod::AnnouncePeer(info_hash) => { 
                        args.info_hash = Some(*info_hash);
                        builder.query_method(Q_ANNOUNCE_PEER)
                    },
                    QueryMethod::Put(data) => {
                        args.bep44 = data.clone();
                        builder.query_method(Q_PUT)
                    },
                    QueryMethod::Get => builder.query_method(Q_GET),
                };
                builder.arguments(args).build()
            },
            MessageKind::Response(r) => {
                let builder = builder.message_type(kmsg::Y_RESPONSE);
                let mut response = response::KResponse::builder().id(self.sender_id).build();
                match r {
                    ResponseKind::Ok => (),
                    ResponseKind::KNearest(nodes) => response.nodes = Some(CompactIPv4NodeInfo { dht_nodes: nodes.clone()}),
                    ResponseKind::Peers(peers) => response.values = Some(peers.iter().map(|p|socket_addr_wrapper::SocketAddrWrapper{ socket_addr: Some(*p)}).collect()),
                    ResponseKind::Data(data) => response.bep44 = data.clone(),
                };
                builder.response(response).build()
            },
            MessageKind::Error(c, m) => {
                builder.message_type(kmsg::Y_ERROR).error(error::Error(*c,m.clone())).build()
            },
        }
    }

    pub fn from_kmsg(kmsg: KMessage) -> Result<Self, SimpleError> {
        let mut own_id = U160::empty();

        let kind = match kmsg.message_type.as_str() {
            kmsg::Y_ERROR => MessageKind::from_kerror(
                kmsg.error
                    .ok_or(simple_error!("stated error type but no error data"))?,
            ),
            kmsg::Y_QUERY => {
                let err = simple_error!("stated query type but no query data");
                if let Some(args) = kmsg.arguments {
                    own_id = args.id;
                    MessageKind::Query(match kmsg.query_method.ok_or(err.clone())?.as_str() {
                        kmsg::Q_PING => QueryMethod::Ping,
                        kmsg::Q_FIND_NODE => QueryMethod::FindNode(args.target.ok_or(err)?),
                        kmsg::Q_ANNOUNCE_PEER => QueryMethod::AnnouncePeer(args.info_hash.ok_or(err)?),
                        kmsg::Q_GET_PEERS => QueryMethod::GetPeers(args.info_hash.ok_or(err)?),
                        kmsg::Q_PUT => QueryMethod::Put(args.bep44),
                        kmsg::Q_GET => QueryMethod::Get,
                        _ => bail!("unknown query type"),
                    })
                } else {
                    bail!(err)
                }
            }
            kmsg::Y_RESPONSE => {
                let err = simple_error!("stated response type but no response data");
                if let Some(response) = kmsg.response {
                    own_id = response.id;
                    MessageKind::Response(if let Some(nodes) = response.nodes {
                        ResponseKind::KNearest(nodes.dht_nodes)
                    } else if let Some(peers) = response.values {
                        ResponseKind::Peers(peers.iter().filter_map(|p| p.socket_addr).collect())
                    } else if response.bep44.v.is_some() {
                        ResponseKind::Data(response.bep44)
                    } else {
                        ResponseKind::Ok
                    })
                } else {
                    bail!(err)
                }
            }
            _ => bail!("Unknown Message Type"),
        };

        Ok(Message {
            received_from_addr: None,
            transaction_id: kmsg.transaction_id,
            sender_id: own_id,
            kind,
            destination_addr: if let Some(wrap) = kmsg.peer_ip {
                wrap.socket_addr
            } else {
                None
            },
            read_only: if let Some(ro) = kmsg.read_only {
                ro
            } else {
                false
            },
        })
    }
}

impl Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageKind::Query(q) => match q {
                QueryMethod::Ping => write!(f,"Ping"),
                QueryMethod::FindNode(_) => write!(f,"Find Node Query"),
                QueryMethod::GetPeers(_) => write!(f,"Get Peers Query"),
                QueryMethod::AnnouncePeer(_) => write!(f,"Announce Query"),
                QueryMethod::Put(_) => write!(f,"Put Data Query"),
                QueryMethod::Get => write!(f,"Get Data Query"),
            },
            MessageKind::Response(r) => match r {
                ResponseKind::Ok => write!(f,"Ok Response"),
                ResponseKind::KNearest(_) => write!(f,"K Nearest Response"),
                ResponseKind::Peers(_) => write!(f,"Peer Response"),
                ResponseKind::Data(_) => write!(f,"Data Response"),
            },
            MessageKind::Error(n, _) => write!(f,"Error {}",n),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils;
    use super::*;

    #[test]
    pub fn ping() {
        let msg = Message::builder()
        .sender_id(u160::U160::rand())
        .transaction_id("test".to_string())
        .destination_addr(<std::net::SocketAddrV4 as std::str::FromStr>::from_str("127.0.0.1:1337").unwrap().into())
        .kind(MessageKind::Query(QueryMethod::Ping))
        .read_only()
        .build();

        let msg = bt_bencode::to_vec(&msg).unwrap();
        println!("bencode: {}", utils::safe_string_from_slice(&msg));

        let msg:Message = bt_bencode::from_slice(&msg).unwrap();
        println!("{:?}", msg);
    }
}