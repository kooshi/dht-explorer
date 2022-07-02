use super::message::*;
use super::*;
use crate::node_info::{self, NodeInfo};
use crate::u160::U160;
use std::str::FromStr;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ping() {
    let client_addr = "127.0.0.1:54321";
    let client_id = U160::rand();
    let client_node = node_info::NodeInfo { id: client_id, addr: SocketAddr::from_str(client_addr).unwrap() };
    let client = Messenger::new(client_node.addr, 100, None, 1).await.unwrap();

    let server_addr = "127.0.0.1:12345";
    let server_id = U160::rand();
    let server_node = node_info::NodeInfo { id: server_id, addr: SocketAddr::from_str(server_addr).unwrap() };
    let handler = Arc::new(TestHandler { info: server_node });
    let _server = Messenger::new(server_node.addr, 100, Some(handler), 1).await.unwrap();

    let querybase = MessageBase {
        origin:         Sender::Me(client_node),
        destination:    Receiver::Node(server_node),
        transaction_id: 123,
        requestor_addr: Some(client_node.addr),
        read_only:      true,
        client:         Client::default(),
    };
    let response = client.query(&querybase.into_query(QueryMethod::Ping)).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert!(!response.read_only);
    assert_eq!(response.kind, ResponseKind::Ok);
    assert_eq!(response.origin.id(), server_id);
    assert_eq!(response.origin.addr(), server_node.addr);
    assert_eq!(response.requestor_addr, Some(client_node.addr));
}

struct TestHandler {
    info: NodeInfo,
}
#[async_trait]
impl QueryHandler for TestHandler {
    async fn handle_query(&self, query: Query) -> QueryResult {
        let returnbase = MessageBase {
            origin:         Sender::Me(self.info),
            destination:    query.origin.into(),
            transaction_id: query.transaction_id,
            requestor_addr: Some(query.origin.addr()),
            read_only:      false,
            client:         Client::default(),
        };
        Ok(returnbase.into_response(ResponseKind::Ok))
    }

    async fn handle_error(&self, tid: u16, to: SocketAddr) -> message::Error {
        let returnbase = MessageBase {
            origin:         Sender::Me(self.info),
            destination:    Receiver::Addr(to),
            transaction_id: tid,
            requestor_addr: Some(to),
            read_only:      false,
            client:         Client::default(),
        };
        returnbase.into_error(KnownError::Server)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn timeout_readonly() {
    let client_addr = SocketAddr::from_str("127.0.0.1:34251").unwrap();
    let client_id = U160::rand();
    let client_node = node_info::NodeInfo { id: client_id, addr: client_addr };
    let client = Messenger::new(client_node.addr, 100, None, 1).await.unwrap();

    let server_addr = "127.0.0.1:15243";
    let server_id = U160::rand();
    let server_node = node_info::NodeInfo { id: server_id, addr: SocketAddr::from_str(server_addr).unwrap() };
    let _ro_server = Messenger::new(server_node.addr, 100, None, 1).await.unwrap();

    let querybase = MessageBase {
        origin:         Sender::Me(client_node),
        destination:    Receiver::Node(server_node),
        transaction_id: 321,
        requestor_addr: Some(client_node.addr),
        read_only:      true,
        client:         Client::default(),
    };
    let response = client.query(&querybase.into_query(QueryMethod::Ping)).await;
    assert!(if let Err(message::Error { code, description, .. }) = &response {
        *code == 201 && description == "Timeout"
    } else {
        false
    });
    let response = response.unwrap_err();
    assert_eq!(response.origin.id(), client_id);
    assert_eq!(response.origin.addr(), client_addr);
    assert_eq!(response.requestor_addr, Some(client_node.addr));
}
