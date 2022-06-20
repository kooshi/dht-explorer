pub(crate) mod kmsg;
mod message_serde;
use kmsg::KMessage;
use simple_error::{bail, simple_error, SimpleError};
use std::{error::Error, net::SocketAddr, ops::Deref};
use typed_builder::TypedBuilder;

use crate::{
    dht_node::DhtNode,
    u160::{self, U160},
};

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub struct Message {
    own_id: U160,
    transaction_id: String,
    #[builder(setter(strip_option))]
    destination_addr: Option<SocketAddr>,
    kind: MessageKind,
    #[builder(setter(strip_bool))]
    read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKind {
    Query(QueryMethod),
    Response(ResponseKind),
    Error(u16, String),
}

impl MessageKind {
    pub fn from_kerror(err: kmsg::error::Error) -> Self {
        MessageKind::Error(err.0, err.1)
    }
}

type InfoHash = U160;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMethod {
    Ping,
    FindNode(U160),
    GetPeers(InfoHash),
    AnnouncePeer(InfoHash),
    Put(kmsg::MessageArgsBep44),
    Get,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseKind {
    Ok,
    KClosest(Vec<DhtNode>),
    Peers(Vec<SocketAddr>),
    Data(kmsg::response::KResponseBep44),
}

impl Message {
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
                        ResponseKind::KClosest(nodes.dht_nodes)
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
            transaction_id: kmsg.transaction_id,
            own_id,
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
