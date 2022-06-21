pub(crate) mod kmsg;
mod message_impl;
use crate::{dht_node::DhtNode, u160::U160};
use std::net::SocketAddr;
use typed_builder::TypedBuilder;

use self::kmsg::nodes::CompactIPv4NodeInfo;

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub struct Message {
    sender_id: U160,
    transaction_id: String,
    #[builder(setter(strip_option))]
    destination_addr: Option<SocketAddr>,
    kind: MessageKind,
    #[builder(setter(strip_bool))]
    read_only: bool,
    #[builder(default, setter(skip))]
    received_from_addr: Option<SocketAddr>,
}

impl Message {
    pub fn sender_id(&self) -> U160 {
        self.sender_id
    }
    pub fn transaction_id(&self) -> &str {
        self.transaction_id.as_str()
    }
    pub fn destination_addr(&self) -> Option<SocketAddr> {
        self.destination_addr
    }
    pub fn kind(&self) -> &MessageKind {
        &self.kind
    }
    pub fn received_from_addr(&self) -> Option<SocketAddr> {
        self.received_from_addr
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKind {
    Query(QueryMethod),
    Response(ResponseKind),
    Error(u16, String),
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
    KNearest(Vec<DhtNode>),
    Peers(Vec<SocketAddr>),
    Data(kmsg::response::KResponseBep44),
}
