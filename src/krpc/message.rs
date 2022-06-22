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
    pub method: QueryMethod,
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

impl From<Query> for Message {
    fn from(item: Query) -> Message {
        Message::Query(item)
    }
}
impl From<Response> for Message {
    fn from(item: Response) -> Message {
        Message::Response(item)
    }
}
impl From<Error> for Message {
    fn from(item: Error) -> Message {
        Message::Error(item)
    }
}

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub struct MessageData {
    pub sender_id: U160,
    pub transaction_id: String,
    #[builder(setter(strip_option))]
    pub destination_addr: Option<SocketAddr>,
    #[builder(setter(strip_bool))]
    pub read_only: bool,
    #[builder(default, setter(skip))]
    pub received_from_addr: Option<SocketAddr>,
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
