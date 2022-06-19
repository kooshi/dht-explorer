


/*

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Ord, PartialOrd, Eq)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[serde(rename = "announce-list")]

*/

use std::u8;

use serde::{Serialize, Deserialize, Deserializer, ser::SerializeSeq};
use serde_derive::{Serialize, Deserialize};
use log::*;
use crate::dht_node::{self, DhtNode, IPV4_DHT_NODE_BYTES_LEN};

// Msg represents messages that nodes in the network send to each other as specified by the protocol.
// They are also referred to as the KRPC messages.
// There are three types of messages: QUERY, RESPONSE, ERROR
// The message is a dictonary that is then
// "bencoded" (serialization & compression format adopted by the BitTorrent)
// and sent via the UDP connection to peers.
//
// A KRPC message is a single dictionary with two keys common to every message and additional keys depending on the type of message.
// Every message has a key "t" with a string value representing a transaction ID.
// This transaction ID is generated by the querying node and is echoed in the response, so responses
// may be correlated with multiple queries to the same node. The transaction ID should be encoded as a short string of binary numbers, typically 2 characters are enough as they cover 2^16 outstanding queries. The other key contained in every KRPC message is "y" with a single character value describing the type of message. The value of the "y" key is one of "q" for query, "r" for response, or "e" for error.
// 3 message types:  QUERY, RESPONSE, ERROR
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Msg {
    // Query method (one of 4: "ping", "find_node", "get_peers", "announce_peer")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "q")]
    query_method:Option<String>, 

    // named arguments sent with a query
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "a")]
    arguments:Option<MsgArgs>,

    // required: transaction ID
    #[serde(rename = "t")]
    transaction_id:String,

    // required: type of the message: q for QUERY, r for RESPONSE, e for ERROR
    #[serde(rename = "y")]
    message_type:String,

    // RESPONSE type only
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "r")]
    response:Option<Response>,

    // ERROR type only
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "e")]
    error:Option<Error>,

    // bep42: outgoing query: requestor ip, incoming query: our ip accodring to the remote
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "ip")]
    peer_ip:Option<CompactPeer>,

    // bep43: ro is a read only top level field
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "ro")]
    read_only:Option<i64>
}

// MsgArgs are the query arguments.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MsgArgs {
    id: String,                 // ID of the querying Node
    target:String,              // ID of the node sought

    // Senders torrent port
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    port:Option<u16>,           // ""required""

    // Use senders apparent DHT port
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    implied_port:Option<bool>,  // ""required""

    // Token received from an earlier get_peers query
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    token:Option<String>,       // ""required""

    // InfoHash of the torrent
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    info_hash:Option<String>,   // ""required""

    // Data stored in a put message (encoded size < 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    v:Option<String>,                                   

    // Seq of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    seq:Option<i64>,

    // CAS value of the message mutation
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    cas:Option<i64>,

    // ed25519 public key (32 bytes string) of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    k:Option<Vec<u8>>,

    // <optional salt to be appended to "k" when hashing (string) a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    salt:Option<String>,

     // ed25519 signature (64 bytes string)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[serde(rename = "sig")]
    sign:Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Response {
    // ID of the querying node
    id:String,
    
    // K closest nodes to the requested target
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    //#[serde(flatten)]
    nodes: Option<CompactIPv4NodeInfo>,

    // Token for future announce_peer
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    token:Option<String>,

    // Torrent peers
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    values:Option<Vec<CompactPeer>>,

    // Data stored in a put message (encoded size < 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    v:Option<String>,

    // Seq of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    seq:Option<i64>,

    // ed25519 public key (32 bytes string) of a mutable msg
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    k:Option<Vec<u8>>,

    // ed25519 signature (64 bytes string)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    #[serde(rename = "sig")]
    sign:Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Error {

}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompactPeer {

}

#[derive(Debug, Clone, /*Serialize, Deserialize,*/ Default, PartialEq, Eq)]
pub struct CompactIPv4NodeInfo {
    // #[serde(skip_serializing_if = "Option::is_none")]
    // #[serde(with = "serde_bytes")]
    // node_bytes:Option<Vec<u8>>,
    
    // #[serde(skip)]
    dht_nodes:Vec<DhtNode>,
}

impl Serialize for CompactIPv4NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let bytes = self.dht_nodes.iter().map(|n|n.ip4_to_bytes()).collect::<Vec<[u8;26]>>().concat();
            serializer.serialize_bytes(&bytes)
    }
}
impl<'de> Deserialize<'de> for CompactIPv4NodeInfo {
    fn deserialize<D>(deserializer: D) -> Result<CompactIPv4NodeInfo, D::Error>
    where
        D: Deserializer<'de> {
            struct CompactIPv4NodeInfoVisitor {}
            impl<'de> serde::de::Visitor<'de> for CompactIPv4NodeInfoVisitor {   
                type Value = CompactIPv4NodeInfo;
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str(&format!("expected n * {} bytes", IPV4_DHT_NODE_BYTES_LEN))
                }
                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                    Ok(CompactIPv4NodeInfo{
                        dht_nodes:v
                        .chunks_exact(IPV4_DHT_NODE_BYTES_LEN)
                        .map(|c|{
                            let mut sized = [0u8;IPV4_DHT_NODE_BYTES_LEN];
                            sized.copy_from_slice(c);
                            DhtNode::ip4_from_bytes(&sized)
                        }).collect()
                    })
                }
            }
            deserializer.deserialize_bytes(CompactIPv4NodeInfoVisitor {})
    }
}


#[cfg(test)]
mod tests {
    use std::{net::SocketAddrV4, str::FromStr};

    use super::*;
    use crate::u160::U160;

    #[test]
    pub fn find_node(){
        let msg = serde_bencoded::from_str::<Msg>("d1:ad2:id20:abcdefghij01234567896:target20:mnopqrstuvwxyz123456e1:q9:find_node1:t2:aa1:y1:qe").unwrap();
        print!("{:?}", msg);
    }

    #[test]
    pub fn response(){
        let response = serde_bencoded::from_str::<Msg>("d1:rd2:id20:0123456789abcdefghij5:nodes9:def456...e1:t2:aa1:y1:re").unwrap();
        print!("{:?}", response);
    }

    #[test]
    pub fn ser_nodes() {
        let socket = std::net::SocketAddr::from(SocketAddrV4::from_str("127.0.0.1:1337").unwrap());
        let host = DhtNode { id: U160::new(), addr: socket };
        let mut nodes = CompactIPv4NodeInfo { dht_nodes: Default::default()};
        nodes.dht_nodes.push(host);
        let host = DhtNode { id: U160::new(), addr: socket };
        nodes.dht_nodes.push(host);
        let ser_nodes = serde_bencoded::to_vec(&nodes).unwrap();
        println!("{}",String::from_utf8_lossy(&ser_nodes));

        let deser_nodes = serde_bencoded::from_bytes::<CompactIPv4NodeInfo>(&ser_nodes).unwrap();
        println!("{:?}", deser_nodes);
        assert_eq!(nodes, deser_nodes);
    }
}