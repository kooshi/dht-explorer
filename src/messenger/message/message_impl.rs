use super::kmsg::wrappers::{CompactInfoHashes, SocketAddrWrapper};
use super::kmsg::*;
use super::*;
use log::error;
use serde::Serialize;
use simple_error::{bail, map_err_with, SimpleResult};
use std::fmt::Display;

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

    pub fn receive(bytes: &[u8], from: SocketAddr) -> Result<Self, (Option<KMessage>, KnownError)> {
        let k: KMessage = bt_bencode::from_slice(bytes).map_err(|e| {
            error!("error deserializing message: {e}");
            (None, KnownError::Server)
        })?;
        let kclone = k.clone();
        Self::from_kmsg(from, k).map_err(|e| (Some(kclone), e))
    }

    pub fn to_bytes(&self) -> SimpleResult<Vec<u8>> {
        map_err_with!(bt_bencode::to_vec(self), "error serializing message")
    }

    pub fn to_kmsg(&self) -> KMessage {
        let builder = KMessage::builder()
            .transaction_id(self.transaction_id.clone())
            .requestor_ip(SocketAddrWrapper {
                socket_addr: match self {
                    Message::Query(_) => Some(self.origin.addr()),
                    Message::Response(_) => self.requestor_addr,
                    Message::Error(_) => None,
                },
            })
            .read_only(self.read_only);
        match &self {
            Message::Query(q) => {
                let mut args = kmsg::MessageArgs::builder().id(self.origin.id()).build();
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
                    QueryMethod::AnnouncePeer { info_hash, port, token } => {
                        args.info_hash = Some(*info_hash);
                        args.token = Some(token.clone());
                        args.port = Some(*port);
                        builder.query_method(Q_ANNOUNCE_PEER)
                    }
                    QueryMethod::Put(base) => {
                        args.bep44 = base.clone();
                        builder.query_method(Q_PUT)
                    }
                    QueryMethod::Get => builder.query_method(Q_GET),
                    QueryMethod::SampleInfohashes(target) => {
                        args.target = Some(*target);
                        builder.query_method(Q_SAMPLE_INFOHASHES)
                    }
                };
                builder.arguments(args).build()
            }
            Message::Response(r) => {
                let builder = builder.message_type(kmsg::Y_RESPONSE);
                let mut response = kresponse::KResponse::builder().id(self.origin.id()).build();
                match &r.kind {
                    ResponseKind::Ok => (),
                    ResponseKind::KNearest { nodes, token } => {
                        response.nodes = Some(CompactIPv4NodeInfo { dht_nodes: nodes.clone() });
                        response.token = token.clone();
                    }
                    ResponseKind::Peers { peers, token } => {
                        response.values =
                            Some(peers.iter().map(|p| SocketAddrWrapper { socket_addr: Some(*p) }).collect());
                        response.token = Some(token.clone());
                    }
                    ResponseKind::Data(base) => response.bep44 = base.clone(),
                    ResponseKind::Samples { nodes, samples, available, interval } => {
                        response.nodes = Some(CompactIPv4NodeInfo { dht_nodes: nodes.clone() });
                        response.bep51.samples = Some(CompactInfoHashes { info_hashes: samples.to_vec() });
                        response.bep51.num = Some(*available);
                        response.bep51.interval = Some(*interval);
                    }
                };
                builder.response(response).build()
            }
            Message::Error(e) =>
                builder.message_type(kmsg::Y_ERROR).error(error::Error(e.error.0, e.error.1.clone())).build(),
        }
    }

    pub fn from_kmsg(origin_addr: SocketAddr, kmsg: KMessage) -> Result<Self, KnownError> {
        let mut base = MessageBase {
            origin:         Sender::Remote(NodeInfo { id: U160::empty(), addr: origin_addr }),
            destination:    Receiver::Me,
            transaction_id: kmsg.transaction_id,
            requestor_addr: if let Some(wrap) = kmsg.requestor_ip { wrap.socket_addr } else { None },
            read_only:      if let Some(ro) = kmsg.read_only { ro } else { false },
            client:         kmsg.version.and_then(|v| v.try_into().ok()).map(Client::from_bytes),
        };

        let message = match kmsg.message_type.as_str() {
            kmsg::Y_ERROR => {
                let error = kmsg.error.ok_or(KnownError::Protocol)?;
                Message::Error(Error { error, base })
            }
            kmsg::Y_QUERY => {
                let err = KnownError::Protocol;
                if let Some(args) = kmsg.arguments {
                    base.origin = Sender::Remote(NodeInfo { id: args.id, addr: origin_addr });
                    Message::Query(Query {
                        base,
                        method: match kmsg.query_method.ok_or(err)?.as_str() {
                            kmsg::Q_PING => QueryMethod::Ping,
                            kmsg::Q_FIND_NODE => QueryMethod::FindNode(args.target.ok_or(err)?),
                            kmsg::Q_ANNOUNCE_PEER => QueryMethod::AnnouncePeer {
                                info_hash: args.info_hash.ok_or(err)?,
                                token:     args.token.ok_or(err)?,
                                port:      if args.implied_port.unwrap_or(false) || args.port.is_none() {
                                    origin_addr.port()
                                } else {
                                    args.port.unwrap()
                                },
                            },
                            kmsg::Q_GET_PEERS => QueryMethod::GetPeers(args.info_hash.ok_or(err)?),
                            kmsg::Q_PUT => QueryMethod::Put(args.bep44),
                            kmsg::Q_GET => QueryMethod::Get,
                            kmsg::Q_SAMPLE_INFOHASHES => QueryMethod::SampleInfohashes(args.target.ok_or(err)?),
                            m => {
                                error!("Got unknown method {m}");
                                return Err(crate::messenger::message::KnownError::MethodUnknown);
                            }
                        },
                    })
                } else {
                    bail!(err)
                }
            }
            kmsg::Y_RESPONSE => {
                let err = KnownError::Protocol;
                if let Some(response) = kmsg.response {
                    base.origin = Sender::Remote(NodeInfo { id: response.id, addr: origin_addr });
                    Message::Response(Response {
                        base,
                        kind: if let Some(nodes) = response.nodes {
                            ResponseKind::KNearest { nodes: nodes.dht_nodes, token: response.token }
                        } else if let Some(peers) = response.values {
                            ResponseKind::Peers {
                                peers: peers.iter().filter_map(|p| p.socket_addr).collect(),
                                token: response.token.ok_or(err)?,
                            }
                        } else if response.bep44.v.is_some() {
                            ResponseKind::Data(response.bep44)
                        } else if let Some(CompactInfoHashes { info_hashes }) = response.bep51.samples {
                            ResponseKind::Samples {
                                nodes:     response.nodes.ok_or(err)?.dht_nodes,
                                samples:   info_hashes,
                                available: response.bep51.num.unwrap_or_default(),
                                interval:  response.bep51.interval.unwrap_or_default(),
                            }
                        } else {
                            ResponseKind::Ok
                        },
                    })
                } else {
                    bail!(err)
                }
            }
            _ => return Err(KnownError::Protocol),
        };
        Ok(message)
    }
}

impl Display for Receiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Receiver::Node(n) => write!(f, "{}", n),
            Receiver::Addr(a) => write!(f, "[{}]", a),
            Receiver::Me => write!(f, "[Me]"),
        }
    }
}

impl Display for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sender::Remote(n) => write!(f, "{}", n),
            Sender::Me(_n) => write!(f, "[Me]"),
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let remote = match &self.origin {
            Sender::Remote(n) => format!(" FROM {n}{}", self.client.as_ref().map_or("".to_owned(), |c| format!("{c}"))),
            Sender::Me(_) => format!(" TO   {}", &self.destination),
        };
        write!(f, "[T:{}]{remote} ", hex::encode(&self.transaction_id))?;
        match self {
            Message::Query(q) => match &q.method {
                QueryMethod::Ping => write!(f, "PING"),
                QueryMethod::FindNode(n) => write!(f, "FIND {n}"),
                QueryMethod::GetPeers(i) => write!(f, "GET PEERS {i}"),
                QueryMethod::AnnouncePeer { info_hash, token, .. } =>
                    write!(f, "ANNOUNCES {info_hash} with token {}", base64::encode(token)),
                QueryMethod::Put(_) => write!(f, "PUTs data"),
                QueryMethod::Get => write!(f, "GETs data"),
                QueryMethod::SampleInfohashes(_) => write!(f, "GET SAMPLES"),
            },
            Message::Response(r) => match &r.kind {
                ResponseKind::Ok => write!(f, "OK"),
                ResponseKind::KNearest { nodes: k, token: t } => write!(
                    f,
                    "NEAREST {} nodes{}",
                    k.len(),
                    t.as_ref().map_or("".into(), |t| format!(" and token {}", base64::encode(&t)))
                ),
                ResponseKind::Peers { peers: p, token: t } =>
                    write!(f, "{} PEERS and token {}", p.len(), base64::encode(&t)),
                ResponseKind::Data(_) => write!(f, "some data"),
                ResponseKind::Samples { samples, available, interval, .. } => write!(
                    f,
                    "{} of {} hashes (refresh in {})",
                    samples.len(),
                    available,
                    chrono::Duration::seconds(*interval as i64)
                ),
            },
            Message::Error(e) => write!(f, "{e}"),
        }
    }
}

impl MessageBase {
    pub fn into_error_generic(self, description: &str) -> Error {
        Error { error: kmsg::error::Error(KnownError::Generic as u16, description.to_owned()), base: self }
    }

    pub fn into_error(self, kind: KnownError) -> Error {
        Error { error: kmsg::error::Error(kind as u16, kind.description().to_owned()), base: self }
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
        let me = Sender::Me(NodeInfo { id: U160::rand(), addr });
        let dest = Receiver::Addr(addr);
        let msg = MessageBase::builder()
            .origin(me)
            .transaction_id(b"654".to_vec())
            .requestor_addr(Some(me.addr()))
            .read_only(true)
            .destination(dest)
            .build()
            .into_query(QueryMethod::Ping)
            .into_message();

        let msg = bt_bencode::to_vec(&msg).unwrap();
        println!("bencode: {}", utils::safe_string_from_slice(&msg));

        let msg: Message = Message::receive(&msg, SocketAddr::from_str("127.127.127.127:127").unwrap()).unwrap();
        println!("{:?}", msg);
    }
}
