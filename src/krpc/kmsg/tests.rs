use super::*;
use crate::u160::U160;
#[cfg(test)]
use std::{
    net::{SocketAddrV4, SocketAddrV6},
    str::FromStr,
};

#[test]
pub fn find_node() {
    let msg = bt_bencode::from_slice::<Message>(b"d1:ad2:id20:abcdefghij01234567896:target20:mnopqrstuvwxyz123456e1:q9:find_node1:t2:aa1:y1:qe").unwrap();
    println!("{:?}", msg);
    assert_eq!(msg.message_type, Y_QUERY);
    assert_eq!(msg.transaction_id, "aa");
    assert_eq!(msg.query_method.unwrap(), Q_FIND_NODE);
    assert_eq!(msg.arguments.as_ref().unwrap().id, U160::from_be_bytes(b"abcdefghij0123456789"));
    assert_eq!(
        msg.arguments.as_ref().unwrap().target.unwrap(),
        U160::from_be_bytes(b"mnopqrstuvwxyz123456")
    );
}

#[test]
pub fn response() {
    let msg = bt_bencode::from_slice::<Message>(
        b"d1:rd2:id20:0123456789abcdefghij5:nodes9:def456...e1:t2:aa1:y1:re",
    )
    .unwrap();
    println!("{:?}", msg);
    assert_eq!(msg.message_type, Y_RESPONSE);
    assert_eq!(msg.transaction_id, "aa");
    assert_eq!(msg.response.as_ref().unwrap().id, "0123456789abcdefghij");
}

#[test]
pub fn ser_nodes() {
    let socket = std::net::SocketAddr::from(SocketAddrV4::from_str("127.0.0.1:1337").unwrap());
    let host = DhtNode {
        id: U160::rand(),
        addr: socket,
    };
    let mut nodes = CompactIPv4NodeInfo {
        dht_nodes: Default::default(),
    };
    nodes.dht_nodes.push(host);
    let host = DhtNode {
        id: U160::rand(),
        addr: socket,
    };
    nodes.dht_nodes.push(host);
    let ser_nodes = bt_bencode::to_vec(&nodes).unwrap();
    println!("{}", String::from_utf8_lossy(&ser_nodes));

    let deser_nodes = bt_bencode::from_slice::<CompactIPv4NodeInfo>(&ser_nodes).unwrap();
    println!("{:?}", deser_nodes);
    assert_eq!(nodes, deser_nodes);
}

#[test]
pub fn error() {
    let test_error = Error::error_invalid_sig();
    let encoded = bt_bencode::to_vec(&test_error).unwrap();
    println!("{:?}", String::from_utf8_lossy(&encoded));

    let error = bt_bencode::from_slice::<Error>(&encoded).unwrap();
    println!("{:?}", error);
    assert_eq!(test_error, error);

    let mut test_msg: Message = Default::default();
    test_msg.message_type = Y_ERROR.to_string();
    test_msg.transaction_id = "aa".to_string();
    test_msg.error = Some(error::Error(201, "A Generic Error Ocurred".to_string()));

    //"d1:eli201e23:A Generic Error Ocurrede1:t2:aa1:y1:ee"
    let test_error = bt_bencode::to_vec(&test_msg).unwrap();
    println!("{:?}", String::from_utf8_lossy(&test_error));

    let err_message = bt_bencode::from_slice::<Message>(&test_error).unwrap();
    println!("{:?}", err_message);
    assert_eq!(err_message, test_msg);
    assert_eq!(err_message.message_type, Y_ERROR);
    assert_eq!(err_message.transaction_id, "aa");
}

#[test]
pub fn addr_wrap() {
    let test = SocketAddrWrapper {
        socket_addr: Some(SocketAddrV4::from_str("127.0.0.1:1337").unwrap().into()),
    };
    let test_vec = bt_bencode::to_vec(&test).unwrap();
    println!("{}", String::from_utf8_lossy(&test_vec));
    let test_out = bt_bencode::from_slice::<SocketAddrWrapper>(&test_vec).unwrap();
    println!("{:?}", test_out);
    assert_eq!(test, test_out);

    let testv6 = SocketAddrWrapper {
        socket_addr: Some(
            SocketAddrV6::from_str("[2001:db8:85a3:8d3:1319:8a2e:370:7348]:1337")
                .unwrap()
                .into(),
        ),
    };
    let test_vec = bt_bencode::to_vec(&testv6).unwrap();
    println!("{}", String::from_utf8_lossy(&test_vec));
    let test_out = bt_bencode::from_slice::<SocketAddrWrapper>(&test_vec).unwrap();
    println!("{:?}", test_out);
    assert_eq!(testv6, test_out);
}
