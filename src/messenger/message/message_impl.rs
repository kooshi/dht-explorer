use super::kmsg::socket_addr_wrapper::SocketAddrWrapper;
use super::kmsg::*;
use super::*;
use serde::Serialize;
use simple_error::{bail, map_err_with, simple_error, try_with, SimpleResult};
use std::fmt::Display;
use std::str::FromStr;

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        self.to_kmsg().serialize(serializer)
    }
}

impl Deref for Query {
    type Target = MessageBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl Deref for Response {
    type Target = MessageBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl Deref for Error {
    type Target = MessageBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl Deref for Message {
    type Target = MessageBase;

    fn deref(&self) -> &Self::Target {
        self.base()
    }
}
impl IMessageBase for Message {
    fn base(&self) -> &MessageBase {
        match self {
            Message::Query(Query { base, .. }) => base,
            Message::Response(Response { base, .. }) => base,
            Message::Error(Error { base, .. }) => base,
        }
    }
}
impl Message {
    fn base_mut(&mut self) -> &mut MessageBase {
        match self {
            Message::Query(Query { base, .. }) => base,
            Message::Response(Response { base, .. }) => base,
            Message::Error(Error { base, .. }) => base,
        }
    }

    pub fn receive(bytes: &[u8], from: SocketAddr) -> SimpleResult<Self> {
        let k: KMessage = try_with!(bt_bencode::from_slice(bytes), "error deserializing message");
        Self::from_kmsg(from, k)
    }

    pub fn to_bytes(&self) -> SimpleResult<Vec<u8>> {
        map_err_with!(bt_bencode::to_vec(self), "error serializing message")
    }

    pub fn to_kmsg(&self) -> KMessage {
        let builder = KMessage::builder()
            .transaction_id(self.transaction_id.to_be_bytes().to_vec())
            .requestor_ip(socket_addr_wrapper::SocketAddrWrapper {
                socket_addr: match self {
                    Message::Query(_) => Some(self.origin.addr),
                    Message::Response(_) => self.requestor_addr,
                    Message::Error(_) => None,
                },
            })
            .read_only(self.read_only);
        match &self {
            Message::Query(q) => {
                let mut args = kmsg::MessageArgs::builder().id(self.origin.id).build();
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
                    QueryMethod::Put(base) => {
                        args.bep44 = base.clone();
                        builder.query_method(Q_PUT)
                    }
                    QueryMethod::Get => builder.query_method(Q_GET),
                };
                builder.arguments(args).build()
            }
            Message::Response(r) => {
                let builder = builder.message_type(kmsg::Y_RESPONSE);
                let mut response = response::KResponse::builder().id(self.origin.id).build();
                match &r.kind {
                    ResponseKind::Ok => (),
                    ResponseKind::KNearest(nodes) =>
                        response.nodes = Some(CompactIPv4NodeInfo { dht_nodes: nodes.clone() }),
                    ResponseKind::Peers(peers) =>
                        response.values =
                            Some(peers.iter().map(|p| SocketAddrWrapper { socket_addr: Some(*p) }).collect()),
                    ResponseKind::Data(base) => response.bep44 = base.clone(),
                };
                builder.response(response).build()
            }
            Message::Error(e) =>
                builder.message_type(kmsg::Y_ERROR).error(error::Error(e.code, e.description.clone())).build(),
        }
    }

    pub fn from_kmsg(origin_addr: SocketAddr, kmsg: KMessage) -> SimpleResult<Self> {
        let mut base = MessageBase {
            origin:         NodeInfo { id: U160::empty(), addr: origin_addr },
            destination:    SocketAddr::from_str("127.0.0.1:1337").unwrap(),
            transaction_id: kmsg.transaction_id.as_chunks().0.iter().next().map_or(0, |c| u16::from_be_bytes(*c)),
            requestor_addr: if let Some(wrap) = kmsg.requestor_ip { wrap.socket_addr } else { None },
            read_only:      if let Some(ro) = kmsg.read_only { ro } else { false },
            client:         kmsg
                .version
                .and_then(|v| v.try_into().ok())
                .map_or_else(Client::default, Client::from_bytes),
        };

        let message = match kmsg.message_type.as_str() {
            kmsg::Y_ERROR => {
                let err = kmsg.error.ok_or_else(|| simple_error!("stated error type but no error data"))?;
                Message::Error(Error { code: err.0, description: err.1, base })
            }
            kmsg::Y_QUERY => {
                let err = simple_error!("stated query type but no query data");
                if let Some(args) = kmsg.arguments {
                    base.origin = NodeInfo { id: args.id, addr: origin_addr };
                    Message::Query(Query {
                        base,
                        method: match kmsg.query_method.ok_or_else(|| err.clone())?.as_str() {
                            kmsg::Q_PING => QueryMethod::Ping,
                            kmsg::Q_FIND_NODE => QueryMethod::FindNode(args.target.ok_or(err)?),
                            kmsg::Q_ANNOUNCE_PEER => QueryMethod::AnnouncePeer(args.info_hash.ok_or(err)?),
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
                    base.origin = NodeInfo { id: response.id, addr: origin_addr };
                    Message::Response(Response {
                        base,
                        kind: if let Some(nodes) = response.nodes {
                            ResponseKind::KNearest(nodes.dht_nodes)
                        } else if let Some(peers) = response.values {
                            ResponseKind::Peers(peers.iter().filter_map(|p| p.socket_addr).collect())
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

impl MessageBase {
    pub fn into_error_generic(self, description: &str) -> Error {
        Error { code: KnownError::Generic as u16, description: description.to_owned(), base: self }
    }

    pub fn into_error(self, kind: KnownError) -> Error {
        Error { code: kind as u16, description: kind.description().to_owned(), base: self }
    }

    pub fn into_query(self, method: QueryMethod) -> Query {
        Query { method, base: self }
    }

    pub fn into_response(self, kind: ResponseKind) -> Response {
        Response { kind, base: self }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils;
    use std::str::FromStr;

    #[test]
    pub fn ping() {
        let addr = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let msg = MessageBase::builder()
            .origin(NodeInfo { id: U160::rand(), addr })
            .transaction_id(654)
            .requestor_addr(addr)
            .read_only(true)
            .destination(addr)
            .build()
            .into_query(QueryMethod::Ping)
            .into_message();

        let msg = bt_bencode::to_vec(&msg).unwrap();
        println!("bencode: {}", utils::safe_string_from_slice(&msg));

        let msg: Message = Message::receive(&msg, SocketAddr::from_str("127.127.127.127:127").unwrap()).unwrap();
        println!("{:?}", msg);
    }
}
