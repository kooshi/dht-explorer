pub(crate) mod kmsg;
mod message_impl;
use self::kmsg::nodes::CompactIPv4NodeInfo;
use crate::node_info::NodeInfo;
use crate::u160::U160;
use std::fmt::Display;
use std::net::SocketAddr;
use std::ops::Deref;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Query(Query),
    Response(Response),
    Error(Error),
}

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub struct MessageBase {
    pub origin:         Sender,
    pub destination:    Receiver,
    pub requestor_addr: Option<SocketAddr>,
    pub transaction_id: Vec<u8>,
    #[builder(default)]
    pub read_only:      bool,
    #[builder(default)]
    pub client:         Option<Client>,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Sender {
    Remote(NodeInfo),
    Me(NodeInfo),
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Receiver {
    Node(NodeInfo),
    Addr(SocketAddr),
    Me,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub method: QueryMethod,
    base:       MessageBase,
}
type InfoHash = U160;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMethod {
    Ping,
    FindNode(U160),
    GetPeers(InfoHash),
    AnnouncePeer { info_hash: InfoHash, port: u16, token: Vec<u8> },
    Put(kmsg::MessageArgsBep44),
    Get,
}

pub type QueryResult = Result<Response, Error>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub kind: ResponseKind,
    base:     MessageBase,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseKind {
    Ok,
    KNearest { nodes: Vec<NodeInfo>, token: Option<Vec<u8>> },
    Peers { peers: Vec<SocketAddr>, token: Vec<u8> },
    Data(kmsg::response::KResponseBep44),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub error: kmsg::error::Error,
    base:      MessageBase,
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
        write!(f, r#"({}) "{}""#, self.error.0, self.error.1)
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
impl Sender {
    pub fn addr(&self) -> SocketAddr {
        match self {
            Sender::Remote(r) => r.addr,
            Sender::Me(m) => m.addr,
        }
    }

    pub fn id(&self) -> U160 {
        match self {
            Sender::Remote(r) => r.id,
            Sender::Me(m) => m.id,
        }
    }
}
impl From<Sender> for Receiver {
    fn from(s: Sender) -> Self {
        match s {
            Sender::Remote(n) => Receiver::Node(n),
            Sender::Me(n) => Receiver::Node(n),
        }
    }
}
impl From<Sender> for NodeInfo {
    fn from(s: Sender) -> Self {
        match s {
            Sender::Remote(n) => n,
            Sender::Me(n) => n,
        }
    }
}
impl From<NodeInfo> for Receiver {
    fn from(n: NodeInfo) -> Self {
        Receiver::Node(n)
    }
}
impl From<Receiver> for Option<SocketAddr> {
    fn from(r: Receiver) -> Self {
        match r {
            Receiver::Node(n) => Some(n.addr),
            Receiver::Addr(a) => Some(a),
            Receiver::Me => None,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct Client {
    name:    &'static str,
    code:    [char; 2],
    version: u16,
}

impl Client {
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut bytes = [0u8; 4];
        bytes[0] = self.code[0] as u8;
        bytes[1] = self.code[1] as u8;
        bytes[2] = (self.version >> 8) as u8;
        bytes[3] = (self.version & 0xff) as u8;
        bytes
    }

    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        let code = [bytes[0] as char, bytes[1] as char];
        let version = (bytes[2] as u16) << 8 | (bytes[3] as u16);
        Client {
            name: match &bytes[..2] {
                b"AG" => "Ares",
                b"A~" => "Ares",
                b"AR" => "Arctic",
                b"AV" => "Avicora",
                b"AX" => "BitPump",
                b"AZ" => "Azureus",
                b"BB" => "BitBuddy",
                b"BC" => "BitComet",
                b"BF" => "Bitflu",
                b"BG" => "BTG (uses Rasterbar libtorrent)",
                b"BR" => "BitRocket",
                b"BS" => "BTSlave",
                b"BX" => "~Bittorrent X",
                b"CD" => "Enhanced CTorrent",
                b"CT" => "CTorrent",
                b"DE" => "DelugeTorrent",
                b"DP" => "Propagate Data Client",
                b"EB" => "EBit",
                b"ES" => "electric sheep",
                b"FT" => "FoxTorrent",
                b"FW" => "FrostWire",
                b"FX" => "Freebox BitTorrent",
                b"GS" => "GSTorrent",
                b"HL" => "Halite",
                b"HN" => "Hydranode",
                b"KG" => "KGet",
                b"KT" => "KTorrent",
                b"LH" => "LH-ABC",
                b"LP" => "Lphant",
                b"LT" => "libtorrent",
                b"lt" => "libTorrent",
                b"LW" => "LimeWire",
                b"MO" => "MonoTorrent",
                b"MP" => "MooPolice",
                b"MR" => "Miro",
                b"MT" => "MoonlightTorrent",
                b"NX" => "Net Transport",
                b"PD" => "Pando",
                b"qB" => "qBittorrent",
                b"QD" => "QQDownload",
                b"QT" => "Qt 4 Torrent example",
                b"RT" => "Retriever",
                b"S~" => "Shareaza alpha/beta",
                b"SB" => "~Swiftbit",
                b"SS" => "SwarmScope",
                b"ST" => "SymTorrent",
                b"st" => "sharktorrent",
                b"SZ" => "Shareaza",
                b"TN" => "TorrentDotNET",
                b"TR" => "Transmission",
                b"TS" => "Torrentstorm",
                b"TT" => "TuoTu",
                b"UL" => "uLeecher!",
                b"UT" => "µTorrent",
                b"UW" => "µTorrent Web",
                b"VG" => "Vagaa",
                b"WD" => "WebTorrent Desktop",
                b"WT" => "BitLet",
                b"WW" => "WebTorrent",
                b"WY" => "FireTorrent",
                b"XL" => "Xunlei",
                b"XT" => "XanTorrent",
                b"XX" => "Xtorrent",
                b"ZT" => "ZipTorrent",
                _ => "Unknown",
            },
            code,
            version,
        }
    }
}
impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let disp = if self.name == "Unknown" { String::from_iter(self.code.iter()) } else { self.name.to_owned() };
        write!(f, "({disp})")
    }
}
