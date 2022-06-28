pub(crate) mod kmsg;
mod message_impl;
use self::kmsg::nodes::CompactIPv4NodeInfo;
use crate::{node_info::NodeInfo, u160::U160};
use std::{fmt::Display, net::SocketAddr, ops::Deref};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Query(Query),
    Response(Response),
    Error(Error),
}

pub type QueryResult = Result<Response, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub method: QueryMethod,
    base:       MessageBase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub kind: ResponseKind,
    base:     MessageBase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code:        u16,
    pub description: String,
    base:            MessageBase,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KnownError {
    Generic            = 201,
    Server             = 202,
    Protocol           = 203,
    MethodUnknown      = 204,
    InvalidV           = 205,
    InvalidSig         = 206,
    SaltTooLong        = 207,
    CasMismatch        = 301,
    SeqLessThanCurrent = 302,
    InvalidNodeId      = 305,
    Internal           = 501,
}

impl KnownError {
    pub fn description(&self) -> &str {
        match self {
            KnownError::Generic => "A Generic Error Occurred",
            KnownError::Server => "The Server Encountered an Error",
            KnownError::Protocol => "Protocol Error, such as a malformed packet, invalid arguments, or bad token",
            KnownError::MethodUnknown => "Method Unknown",
            KnownError::InvalidV => "V missing or too long (>999).",
            KnownError::InvalidSig => "Invalid signature",
            KnownError::SaltTooLong => "Salt too long (>64)",
            KnownError::CasMismatch => "the CAS hash didn't match, re-read value and try again",
            KnownError::SeqLessThanCurrent => "sequence number less than current",
            KnownError::Internal => "An internal error occurred",
            KnownError::InvalidNodeId => "Invalid node id",
        }
    }
}

impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"({}) "{}""#, self.code, self.description)
    }
}

impl Query {
    pub fn into_message(self) -> Message {
        Message::Query(self)
    }
}
impl From<QueryResult> for Message {
    fn from(r: QueryResult) -> Self {
        r.map_or_else(|e| e.into_message(), |r| r.into_message())
    }
}
impl Response {
    pub fn into_message(self) -> Message {
        Message::Response(self)
    }
}
impl Error {
    pub fn into_message(self) -> Message {
        Message::Error(self)
    }
}
pub trait IMessageBase {
    fn base(&self) -> &MessageBase;
}
impl IMessageBase for QueryResult {
    fn base(&self) -> &MessageBase {
        match self {
            Ok(r) => &r.base,
            Err(e) => &e.base,
        }
    }
}

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub struct MessageBase {
    pub origin:           NodeInfo,
    pub transaction_id:   u16,
    #[builder(setter(strip_option))]
    pub destination_addr: Option<SocketAddr>,
    #[builder(default)]
    pub read_only:        bool,
    //pub version:            [u8; 4],
}

// pub struct Version {
//     client: String,
//     verison:
// }

type InfoHash = U160;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMethod {
    Ping,
    FindNode(U160),
    GetPeers(InfoHash),
    AnnouncePeer(InfoHash), //add implied port
    Put(kmsg::MessageArgsBep44),
    Get,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseKind {
    Ok,
    KNearest(Vec<NodeInfo>),
    Peers(Vec<SocketAddr>),
    Data(kmsg::response::KResponseBep44),
}
