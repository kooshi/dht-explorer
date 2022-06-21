pub(crate) mod kmsg;
mod message_impl;
use crate::{dht_node::DhtNode, u160::U160};
use std::{net::SocketAddr, ops::Deref};
use typed_builder::TypedBuilder;

use self::kmsg::nodes::CompactIPv4NodeInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Query(Query),
    Response(Response),
    Error(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    method: QueryMethod,
    data: MessageData,
}
impl Query {
    pub fn new(method: QueryMethod, data: MessageData) -> Self {
        Self { method, data }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    kind: ResponseKind,
    data: MessageData,
}
impl Response {
    pub fn new(kind: ResponseKind, data: MessageData) -> Self {
        Self { kind, data }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: u16,
    description: String,
    data: MessageData,
}
impl Error {
    pub fn new(code: u16, description: String, data: MessageData) -> Self {
        Self {
            code,
            description,
            data,
        }
    }
}
pub trait IMessage {
    fn to_message(self) -> Message;
}
impl IMessage for Query {
    fn to_message(self) -> Message {
        Message::Query(self)
    }
}
impl IMessage for Response {
    fn to_message(self) -> Message {
        Message::Response(self)
    }
}
impl IMessage for Error {
    fn to_message(self) -> Message {
        Message::Error(self)
    }
}
impl IMessage for Message {
    fn to_message(self) -> Message {
        self
    }
}

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub struct MessageData {
    sender_id: U160,
    transaction_id: String,
    #[builder(setter(strip_option))]
    destination_addr: Option<SocketAddr>,
    #[builder(setter(strip_bool))]
    read_only: bool,
    #[builder(default, setter(skip))]
    received_from_addr: Option<SocketAddr>,
}

impl MessageData {
    pub fn sender_id(&self) -> U160 {
        self.sender_id
    }
    pub fn transaction_id(&self) -> &str {
        self.transaction_id.as_str()
    }
    pub fn destination_addr(&self) -> Option<SocketAddr> {
        self.destination_addr
    }
    pub fn received_from_addr(&self) -> Option<SocketAddr> {
        self.received_from_addr
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
    KNearest(Vec<DhtNode>),
    Peers(Vec<SocketAddr>),
    Data(kmsg::response::KResponseBep44),
}
