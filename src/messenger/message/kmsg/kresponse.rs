use super::wrappers::CompactInfoHashes;
use super::*;
use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct KResponse {
    // ID of the querying node
    pub id: U160,

    // K closest nodes to the requested target
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub nodes: Option<CompactIPv4NodeInfo>,

    // Token for future announce_peer
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[builder(default, setter(strip_option))]
    pub token: Option<Vec<u8>>,

    // Torrent peers
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub values: Option<Vec<SocketAddrWrapper>>,

    #[serde(flatten)]
    #[serde(default)]
    #[builder(default)]
    pub bep44: KResponseBep44,

    #[serde(flatten)]
    #[serde(default)]
    #[builder(default)]
    pub bep51: ResponseBep51,
}

#[derive(TypedBuilder, Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct KResponseBep44 {
    // Data stored in a put message (encoded size < 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[builder(default, setter(strip_option))]
    pub v: Option<Vec<u8>>,

    // Seq of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub seq: Option<i64>,

    // ed25519 public key (32 bytes string) of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[builder(default, setter(strip_option))]
    pub k: Option<Vec<u8>>,

    // ed25519 signature (64 bytes string)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[builder(default, setter(strip_option))]
    pub sig: Option<Vec<u8>>,
}

#[derive(TypedBuilder, Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ResponseBep51 {
    // the time in seconds that the samples are rotated
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(strip_option))]
    pub interval: Option<u16>,

    // A list of infohashes for which we have stored peers, limited by UDP packet length
    // Nodes supporting this extension should always include the samples field in the response, even
    // when it is zero-length. This lets indexing nodes to distinguish nodes supporting this
    // extension from those that respond to unknown query types which contain a target field [2].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(strip_option))]
    pub samples: Option<CompactInfoHashes>,

    // total number of known infohashes in storage
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(strip_option))]
    pub num: Option<u64>,
}

pub struct ResponseBep33 {
    //TODO https://github.com/anacrolix/dht/blob/master/krpc/msg.go
    //     https://www.bittorrent.org/beps/bep_0033.html
}
