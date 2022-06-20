use super::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Response {
    // ID of the querying node
    pub id: U160,

    // K closest nodes to the requested target
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    //#[serde(flatten)]
    pub nodes: Option<CompactIPv4NodeInfo>,

    // Token for future announce_peer
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub token: Option<String>,

    // Torrent peers
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub values: Option<Vec<SocketAddrWrapper>>,

    #[serde(flatten)]
    pub bep44: ResponseBep44,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ResponseBep44 {
    // Data stored in a put message (encoded size < 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub v: Option<Vec<u8>>,

    // Seq of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub seq: Option<i64>,

    // ed25519 public key (32 bytes string) of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub k: Option<Vec<u8>>,

    // ed25519 signature (64 bytes string)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub sig: Option<Vec<u8>>,
}


pub struct ResponseBep51 {
    //TODO https://github.com/anacrolix/dht/blob/master/krpc/msg.go
    //     https://www.bittorrent.org/beps/bep_0051.html
}

pub struct ResponseBep33 {
    //TODO https://github.com/anacrolix/dht/blob/master/krpc/msg.go
    //     https://www.bittorrent.org/beps/bep_0033.html
}