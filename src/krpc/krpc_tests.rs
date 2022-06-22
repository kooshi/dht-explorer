use super::*;
use crate::{dht_node, krpc::message::Response, u160::U160};
use std::str::FromStr;
use tokio;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ping() {
    let client_addr = "127.0.0.1:54321";
    let client_id = U160::rand();
    let client_node = dht_node::DhtNode { id: client_id, addr: SocketAddr::from_str(client_addr).unwrap() };
    let client = KrpcService::new(client_node, 100, true).await.unwrap();

    let server_addr = "127.0.0.1:12345";
    let server_id = U160::rand();
    let server_node = dht_node::DhtNode { id: server_id, addr: SocketAddr::from_str(server_addr).unwrap() };
    let _server = KrpcService::new(server_node, 100, false).await.unwrap();

    let response = client.query(QueryMethod::Ping, server_node.addr).await.unwrap();
    assert!(if let Message::Response(Response { kind: ResponseKind::Ok, .. }) = response { true } else { false });
    assert_eq!(response.read_only, false);
    assert_eq!(response.sender_id, server_id);
    assert_eq!(response.received_from_addr, Some(server_node.addr));
    assert_eq!(response.destination_addr, Some(client_node.addr));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn timeout_readonly() {
    let client_addr = "127.0.0.1:34251";
    let client_id = U160::rand();
    let client_node = dht_node::DhtNode { id: client_id, addr: SocketAddr::from_str(client_addr).unwrap() };
    let client = KrpcService::new(client_node, 100, true).await.unwrap();

    let server_addr = "127.0.0.1:15243";
    let server_id = U160::rand();
    let server_node = dht_node::DhtNode { id: server_id, addr: SocketAddr::from_str(server_addr).unwrap() };
    let _ro_server = KrpcService::new(server_node, 100, true).await.unwrap();

    let response = client.query(QueryMethod::Ping, server_node.addr).await.unwrap();
    assert!(if let Message::Error(message::Error { code, description, .. }) = &response {
        *code == 201 && description == "Timeout"
    } else {
        false
    });

    assert_eq!(response.sender_id, client_id);
    assert_eq!(response.received_from_addr, None);
    assert_eq!(response.destination_addr, Some(server_node.addr));
}
