use super::{message::*, *};
use crate::{node_info, u160::U160};
use futures::future::BoxFuture;
use std::str::FromStr;
use tokio;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ping() {
    let client_addr = "127.0.0.1:54321";
    let client_id = U160::rand();
    let client_node = node_info::NodeInfo { id: client_id, addr: SocketAddr::from_str(client_addr).unwrap() };
    let client = Messenger::new(client_node, 100, None).await.unwrap();

    let server_addr = "127.0.0.1:12345";
    let server_id = U160::rand();
    let server_node = node_info::NodeInfo { id: server_id, addr: SocketAddr::from_str(server_addr).unwrap() };
    let _server = Messenger::new(server_node, 100, Some(Box::new(handler))).await.unwrap();

    let response = client.query(QueryMethod::Ping, server_node.addr).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.kind, ResponseKind::Ok);
    assert_eq!(response.read_only, false);
    assert_eq!(response.sender_id, server_id);
    assert_eq!(response.received_from_addr, Some(server_node.addr));
    assert_eq!(response.destination_addr, Some(client_node.addr));
}

fn handler(base: MessageBase, _query: Query) -> BoxFuture<'static, QueryResult> {
    Box::pin(async { Ok(base.to_response(ResponseKind::Ok)) })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn timeout_readonly() {
    let client_addr = "127.0.0.1:34251";
    let client_id = U160::rand();
    let client_node = node_info::NodeInfo { id: client_id, addr: SocketAddr::from_str(client_addr).unwrap() };
    let client = Messenger::new(client_node, 100, None).await.unwrap();

    let server_addr = "127.0.0.1:15243";
    let server_id = U160::rand();
    let server_node = node_info::NodeInfo { id: server_id, addr: SocketAddr::from_str(server_addr).unwrap() };
    let _ro_server = Messenger::new(server_node, 100, None).await.unwrap();

    let response = client.query(QueryMethod::Ping, server_node.addr).await;
    assert!(if let Err(message::Error { code, description, .. }) = &response {
        *code == 201 && description == "Timeout"
    } else {
        false
    });
    let response = response.unwrap_err();
    assert_eq!(response.sender_id, client_id);
    assert_eq!(response.received_from_addr, None);
    assert_eq!(response.destination_addr, Some(server_node.addr));
}
