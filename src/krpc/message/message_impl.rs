use super::{kmsg::*, *};
use serde::{de, Deserialize, Deserializer, Serialize};
use simple_error::{bail, simple_error, SimpleError};
use std::fmt::Display;

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
        Message::from_kmsg(KMessage::deserialize(deserializer)?).map_err(|e| de::Error::custom(e))
    }
}

impl Deref for Query {
    type Target = MessageData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl Deref for Response {
    type Target = MessageData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl Deref for Error {
    type Target = MessageData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl Message {
    fn data_mut(&mut self) -> &mut MessageData {
        match self {
            Message::Query(Query { data, .. }) => data,
            Message::Response(Response { data, .. }) => data,
            Message::Error(Error { data, .. }) => data,
        }
    }
    pub fn data(&self) -> &MessageData {
        match self {
            Message::Query(Query { data, .. }) => &data,
            Message::Response(Response { data, .. }) => &data,
            Message::Error(Error { data, .. }) => &data,
        }
    }

    pub fn receive(bytes: &[u8], from: SocketAddr) -> Result<Self, SimpleError> {
        Self::from_bytes(bytes).map(|mut s| {
            s.data_mut().received_from_addr = Some(from);
            s
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SimpleError> {
        bt_bencode::from_slice(bytes)
            .map_err(|e| SimpleError::with("error getting message from bytes", e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bt_bencode::to_vec(self).unwrap()
    }

    pub fn to_kmsg(&self) -> KMessage {
        let builder = KMessage::builder()
            .transaction_id(self.data().transaction_id.clone())
            .peer_ip(socket_addr_wrapper::SocketAddrWrapper {
                socket_addr: self.data().destination_addr,
            })
            .read_only(self.data().read_only);
        match &self {
            Message::Query(q) => {
                let mut args = kmsg::MessageArgs::builder()
                    .id(self.data().sender_id)
                    .build();
                let builder = builder.message_type(kmsg::Y_QUERY);
                let builder = match &q.method {
                    QueryMethod::Ping => builder.query_method(Q_PING),
                    QueryMethod::FindNode(target) => {
                        args.target = Some(*target);
                        builder.query_method(Q_FIND_NODE)
                    }
                    QueryMethod::GetPeers(info_hash) => {
                        args.info_hash = Some(*info_hash);
                        builder.query_method(Q_GET_PEERS)
                    }
                    QueryMethod::AnnouncePeer(info_hash) => {
                        args.info_hash = Some(*info_hash);
                        builder.query_method(Q_ANNOUNCE_PEER)
                    }
                    QueryMethod::Put(data) => {
                        args.bep44 = data.clone();
                        builder.query_method(Q_PUT)
                    }
                    QueryMethod::Get => builder.query_method(Q_GET),
                };
                builder.arguments(args).build()
            }
            Message::Response(r) => {
                let builder = builder.message_type(kmsg::Y_RESPONSE);
                let mut response = response::KResponse::builder()
                    .id(self.data().sender_id)
                    .build();
                match &r.kind {
                    ResponseKind::Ok => (),
                    ResponseKind::KNearest(nodes) => {
                        response.nodes = Some(CompactIPv4NodeInfo {
                            dht_nodes: nodes.clone(),
                        })
                    }
                    ResponseKind::Peers(peers) => {
                        response.values = Some(
                            peers
                                .iter()
                                .map(|p| socket_addr_wrapper::SocketAddrWrapper {
                                    socket_addr: Some(*p),
                                })
                                .collect(),
                        )
                    }
                    ResponseKind::Data(data) => response.bep44 = data.clone(),
                };
                builder.response(response).build()
            }
            Message::Error(e) => builder
                .message_type(kmsg::Y_ERROR)
                .error(error::Error(e.code, e.description.clone()))
                .build(),
        }
    }

    pub fn from_kmsg(kmsg: KMessage) -> Result<Self, SimpleError> {
        let mut data = MessageData {
            received_from_addr: None,
            transaction_id: kmsg.transaction_id,
            sender_id: U160::empty(),
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
        };

        let message = match kmsg.message_type.as_str() {
            kmsg::Y_ERROR => {
                let err = kmsg
                    .error
                    .ok_or(simple_error!("stated error type but no error data"))?;
                Message::Error(Error {
                    code: err.0,
                    description: err.1,
                    data,
                })
            }
            kmsg::Y_QUERY => {
                let err = simple_error!("stated query type but no query data");
                if let Some(args) = kmsg.arguments {
                    data.sender_id = args.id;
                    Message::Query(Query {
                        data,
                        method: match kmsg.query_method.ok_or(err.clone())?.as_str() {
                            kmsg::Q_PING => QueryMethod::Ping,
                            kmsg::Q_FIND_NODE => QueryMethod::FindNode(args.target.ok_or(err)?),
                            kmsg::Q_ANNOUNCE_PEER => {
                                QueryMethod::AnnouncePeer(args.info_hash.ok_or(err)?)
                            }
                            kmsg::Q_GET_PEERS => QueryMethod::GetPeers(args.info_hash.ok_or(err)?),
                            kmsg::Q_PUT => QueryMethod::Put(args.bep44),
                            kmsg::Q_GET => QueryMethod::Get,
                            _ => bail!("unknown query type"),
                        },
                    })
                } else {
                    bail!(err)
                }
            }
            kmsg::Y_RESPONSE => {
                let err = simple_error!("stated response type but no response data");
                if let Some(response) = kmsg.response {
                    data.sender_id = response.id;
                    Message::Response(Response {
                        data,
                        kind: if let Some(nodes) = response.nodes {
                            ResponseKind::KNearest(nodes.dht_nodes)
                        } else if let Some(peers) = response.values {
                            ResponseKind::Peers(
                                peers.iter().filter_map(|p| p.socket_addr).collect(),
                            )
                        } else if response.bep44.v.is_some() {
                            ResponseKind::Data(response.bep44)
                        } else {
                            ResponseKind::Ok
                        },
                    })
                } else {
                    bail!(err)
                }
            }
            _ => bail!("Unknown Message Type"),
        };
        Ok(message)
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Query(q) => match q.method {
                QueryMethod::Ping => write!(f, "Ping"),
                QueryMethod::FindNode(_) => write!(f, "Find Node Query"),
                QueryMethod::GetPeers(_) => write!(f, "Get Peers Query"),
                QueryMethod::AnnouncePeer(_) => write!(f, "Announce Query"),
                QueryMethod::Put(_) => write!(f, "Put Data Query"),
                QueryMethod::Get => write!(f, "Get Data Query"),
            },
            Message::Response(r) => match r.kind {
                ResponseKind::Ok => write!(f, "Ok Response"),
                ResponseKind::KNearest(_) => write!(f, "K Nearest Response"),
                ResponseKind::Peers(_) => write!(f, "Peer Response"),
                ResponseKind::Data(_) => write!(f, "Data Response"),
            },
            Message::Error(Error { code: n, .. }) => write!(f, "Error {}", n),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils;

    #[test]
    pub fn ping() {
        let data = MessageData::builder()
            .sender_id(U160::rand())
            .transaction_id("test".to_string())
            .destination_addr(
                <std::net::SocketAddrV4 as std::str::FromStr>::from_str("127.0.0.1:1337")
                    .unwrap()
                    .into(),
            )
            .read_only()
            .build();

        let msg = Message::Query(Query {
            data,
            method: QueryMethod::Ping,
        });
        let msg = bt_bencode::to_vec(&msg).unwrap();
        println!("bencode: {}", utils::safe_string_from_slice(&msg));

        let msg: Message = bt_bencode::from_slice(&msg).unwrap();
        println!("{:?}", msg);
    }
}
