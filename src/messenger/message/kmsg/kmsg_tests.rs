use super::*;
use crate::u160::U160;
use crate::utils;
use std::net::SocketAddr;
use std::str::FromStr;
#[test]
pub fn find_node() {
    let msg = bt_bencode::from_slice::<KMessage>(
        b"d1:ad2:id20:abcdefghij01234567896:target20:mnopqrstuvwxyz123456e1:q9:find_node1:t2:aa1:y1:qe",
    )
    .unwrap();
    println!("{:?}", msg);
    assert_eq!(msg.message_type, Y_QUERY);
    assert_eq!(msg.transaction_id, b"aa".to_vec());
    assert_eq!(msg.query_method.unwrap(), Q_FIND_NODE);
    assert_eq!(msg.arguments.as_ref().unwrap().id, U160::from_be_bytes(b"abcdefghij0123456789"));
    assert_eq!(msg.arguments.as_ref().unwrap().target.unwrap(), U160::from_be_bytes(b"mnopqrstuvwxyz123456"));
}

#[test]
pub fn response() {
    let msg = bt_bencode::from_slice::<KMessage>(b"d1:rd2:id20:0123456789abcdefghij5:nodes9:def456...e1:t2:aa1:y1:re")
        .unwrap();
    println!("{:?}", msg);
    assert_eq!(msg.message_type, Y_RESPONSE);
    assert_eq!(msg.transaction_id, b"aa".to_vec());
    assert_eq!(msg.response.as_ref().unwrap().id, U160::from_be_bytes(b"0123456789abcdefghij"));
}

#[test]
pub fn ser_nodes() {
    let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
    let host = NodeInfo { id: U160::rand(), addr: socket };
    let mut nodes = CompactIPv4NodeInfo { dht_nodes: Default::default() };
    nodes.dht_nodes.push(host);
    let host = NodeInfo { id: U160::rand(), addr: socket };
    nodes.dht_nodes.push(host);
    let ser_nodes = bt_bencode::to_vec(&nodes).unwrap();
    println!("NODES: {}", utils::safe_string_from_slice(&ser_nodes));

    let deser_nodes = bt_bencode::from_slice::<CompactIPv4NodeInfo>(&ser_nodes).unwrap();
    println!("{:?}", deser_nodes);
    assert_eq!(nodes, deser_nodes);
}

#[allow(clippy::field_reassign_with_default)]
#[test]
pub fn error() {
    let test_error = Error(201, "Generic".to_owned());
    let encoded = bt_bencode::to_vec(&test_error).unwrap();
    println!("TESTERROR: {}", utils::safe_string_from_slice(&encoded));

    let error = bt_bencode::from_slice::<Error>(&encoded).unwrap();
    println!("{:?}", error);
    assert_eq!(test_error, error);

    let mut test_msg: KMessage = Default::default();
    test_msg.message_type = Y_ERROR.to_string();
    test_msg.transaction_id = b"aa".to_vec();
    test_msg.error = Some(error::Error(201, "A Generic Error Ocurred".to_string()));

    //"d1:eli201e23:A Generic Error Ocurrede1:t2:aa1:y1:ee"
    let test_error = bt_bencode::to_vec(&test_msg).unwrap();
    println!("TESTERROR: {}", utils::safe_string_from_slice(&test_error));

    let err_message = bt_bencode::from_slice::<KMessage>(&test_error).unwrap();
    println!("{:?}", err_message);
    assert_eq!(err_message, test_msg);
    assert_eq!(err_message.message_type, Y_ERROR);
    assert_eq!(err_message.transaction_id, b"aa".to_vec());
}

#[test]
pub fn addr_wrap() {
    let test = SocketAddrWrapper { socket_addr: Some(SocketAddr::from_str("127.0.0.1:1337").unwrap()) };
    let test_vec = bt_bencode::to_vec(&test).unwrap();
    println!("TESTVEC {}", utils::safe_string_from_slice(&test_vec));
    let test_out = bt_bencode::from_slice::<SocketAddrWrapper>(&test_vec).unwrap();
    println!("{:?}", test_out);
    assert_eq!(test, test_out);

    let testv6 = SocketAddrWrapper {
        socket_addr: Some(SocketAddr::from_str("[2001:db8:85a3:8d3:1319:8a2e:370:7348]:1337").unwrap()),
    };
    let test_vec = bt_bencode::to_vec(&testv6).unwrap();
    println!("TESTVEC: {}", utils::safe_string_from_slice(&test_vec));
    let test_out = bt_bencode::from_slice::<SocketAddrWrapper>(&test_vec).unwrap();
    println!("{:?}", test_out);
    assert_eq!(testv6, test_out);
}

#[test]
pub fn find_nodes_response() {
    let buf = base64::decode("ZDE6cmQyOmlkMjA6es6LsAHqL6S93sAyV+y8t2mzqLc1Om5vZGVzMjA4Ohxv57KlTw7ylJm/nb9dlzxDiGb4Xhec08jVHHzKfICCRyerOHPZ5RFX4l8ZVS8Fh7SuyNUt0T6IPmnhch1HBIiumvC3UEJwikgVERrB2C4aUVB3Ct3idsofBy76tWEIvANLwRf6yMfYPMf1EUymrcebpxptH/y4+oL2pppV5GYISj8Ap1bzrrZLAN16RBnANuNDBmZk67mdoUT3cSXUhlwwWTbCY1v+OeOzxg8Ukr35w9ElOsjVHXRLocGoVFTZAfvTeZ5szKs2kjBOUfyP2P9lMTp0Nzp0ZXN0aW5nMTp2NDpsdA2AMTp5MTpyZQ==").unwrap();
    println!("MESSAGE: {}", utils::safe_string_from_slice(&buf));
    let msg = bt_bencode::from_slice::<KMessage>(&buf).unwrap();
    println!("{:?}", msg);
}

#[test]
pub fn ping_response_plus_data() {
    let buf =
        base64::decode("ZDE6cmQyOmlkMjA6es6LsAHqL6S93sAyV+y8t2mzqLdlMTp0Nzp0ZXN0aW5nMTp2NDpsdA2AMTp5MTpyZQ==").unwrap();
    println!("MESSAGE: {}", utils::safe_string_from_slice(&buf));
    let mut msg = bt_bencode::from_slice::<KMessage>(&buf).unwrap();
    msg.response.as_mut().unwrap().bep44.v = Some(b"HELLOWORLD".to_vec());

    let buf = bt_bencode::to_vec(&msg).unwrap();
    println!("MESSAGE: {}", utils::safe_string_from_slice(&buf));

    let msg = bt_bencode::from_slice::<KMessage>(&buf).unwrap();
    println!("{:?}", msg);
}

#[test]
pub fn error_response() {
    let buf =
        base64::decode("ZDE6ZWxpMjAzZTMwOlRyYW5zYWN0aW9uIElEIGxlbmd0aCB0b28gbG9uZ2UxOnY0Omx0DYAxOnkxOmVl").unwrap();
    println!("MESSAGE: {}", utils::safe_string_from_slice(&buf));
    let msg = bt_bencode::from_slice::<KMessage>(&buf).unwrap();
    println!("{:?}", msg);

    println!(
        "{:#?}",
        crate::messenger::message::Message::from_kmsg(SocketAddr::from_str("0.0.0.0:1234").unwrap(), msg)
    );
}

#[test]
pub fn query() {
    let buf =
        base64::decode("ZDE6YWQyOmlkMjA6oGCokH64btALaJnYkDs1B0Sgw0A2OnRhcmdldDIwOqBgrrQAAGxyAAAYaAAAX5QAAG1kZTE6cTk6ZmluZF9ub2RlMTp0NDoBPwAAMTp2NDpVVLTgMTp5MTpxZQ==").unwrap();
    println!("MESSAGE: {}", utils::safe_string_from_slice(&buf));
    let msg = bt_bencode::from_slice::<KMessage>(&buf).unwrap();
    println!("{:?}", msg);

    println!(
        "{:#?}",
        crate::messenger::message::Message::from_kmsg(SocketAddr::from_str("0.0.0.0:1234").unwrap(), msg)
    );
}

#[test]
pub fn query2() {
    let buf =
        base64::decode("ZDE6YWQyOmlkMjA6oGCuq2l6uuzW00L4vlO3t9SZG3JlMTpxNDpwaW5nMTp0MjrhrzE6djQ6TFQCBjE6eTE6cWU=")
            .unwrap();
    println!("MESSAGE: {}", utils::safe_string_from_slice(&buf));
    let msg = bt_bencode::from_slice::<KMessage>(&buf).unwrap();
    println!("{:?}", msg);

    println!(
        "{:#?}",
        crate::messenger::message::Message::from_kmsg(SocketAddr::from_str("0.0.0.0:1234").unwrap(), msg)
    );
}
