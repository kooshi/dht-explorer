use crate::{messenger::{self, message::{KnownError, MessageBase, Query, QueryResult, ResponseKind}, Messenger, QueryHandler, WrappedQueryHandler}, node_info::NodeInfo, router::Router, u160::U160};
use async_trait::async_trait;
use futures::future::join_all;
use log::error;
use messenger::message::QueryMethod;
use simple_error::{try_with, SimpleResult};
use std::{net::SocketAddr, str::FromStr, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

#[derive(Clone)]
pub struct Node {
    messenger: Arc<Messenger>,
    state:     Arc<NodeState>,
}

impl Node {
    pub async fn new(addr: SocketAddr, read_only: bool) -> SimpleResult<Self> {
        let router = Router::new("./target/buckets.ben".into()).await?;
        let state = Arc::new(NodeState { router, transaction: AtomicUsize::new(0), read_only });
        let handler: Option<WrappedQueryHandler> = if read_only { None } else { Some(state.clone()) };
        let messenger = Arc::new(Messenger::new(addr, 500, handler).await?);
        Ok(Node { messenger, state })
    }

    pub async fn bootstrap(&self, bootstrap_node: SocketAddr) -> SimpleResult<()> {
        let response = try_with!(
            self.messenger.query(&self.message_base(bootstrap_node).into_query(QueryMethod::Ping)).await,
            "could not reach bootstrap node"
        );
        self.send_find_node(response.origin, self.state.router.own_id()).await;
        while (self.find_node(self.state.router.own_id()).await).is_none() {
            //self.find_node(U160::rand()).await;
        }
        Ok(())
    }

    pub async fn find_node(&self, id: U160) -> Option<NodeInfo> {
        let closest_known = self.state.router.lookup(id).await;
        if let Some(target) = closest_known.iter().find(|n| n.id == id) {
            return Some(*target);
        }
        let mut joins = Vec::with_capacity(closest_known.len());
        for node in closest_known {
            let clone = self.clone();
            joins.push(tokio::spawn(async move {
                clone.send_find_node(node, id).await;
            }));
        }
        join_all(joins).await;
        None
    }

    async fn send_find_node(&self, to: NodeInfo, id: U160) {
        match self.messenger.query(&self.message_base(to.addr).into_query(QueryMethod::FindNode(id))).await {
            Ok(r) => {
                self.state.router.add(r.origin).await;
                if let ResponseKind::KNearest(nodes) = r.kind {
                    for n in nodes {
                        self.state.router.add(n).await;
                    }
                }
            }
            Err(e) => {
                error!("Received error response: {}", e);
                self.state.router.ban_id(to.id).await;
            }
        }
    }

    fn message_base(&self, to: SocketAddr) -> MessageBase {
        MessageBase::builder()
            .origin(NodeInfo {
                id:   self.state.router.own_id(),
                addr: SocketAddr::from_str("127.0.0.1:1337").unwrap(),
            }) //TODO fix addr later, doesn't really matter
            .transaction_id(self.state.transaction.fetch_add(1, Ordering::Relaxed) as u16)
            .destination_addr(to)
            .read_only(self.state.read_only)
            .build()
    }

    fn own_id(&self) -> U160 {
        self.state.router.own_id()
    }
}

pub struct NodeState {
    router:      Router,
    transaction: AtomicUsize,
    read_only:   bool,
}

#[async_trait]
impl QueryHandler for NodeState {
    async fn handle_query(&self, query: Query) -> QueryResult {
        assert!(!self.read_only);
        if !query.read_only {
            self.router.add(query.origin).await;
        }
        let response_base = MessageBase::builder()
            .origin(NodeInfo { id: self.router.own_id(), addr: SocketAddr::from_str("127.0.0.1:1337").unwrap() }) //TODO fix addr later, doesn't really matter
            .transaction_id(query.transaction_id.clone())
            .destination_addr(query.origin.addr)
            .build();
        match query.method {
            QueryMethod::Ping => Ok(response_base.into_response(ResponseKind::Ok)),
            QueryMethod::FindNode(n) =>
                Ok(response_base.into_response(ResponseKind::KNearest(self.router.lookup(n).await))),
            QueryMethod::GetPeers(n) =>
                Ok(response_base.into_response(ResponseKind::KNearest(self.router.lookup(n).await))),
            QueryMethod::AnnouncePeer(_) => Err(response_base.into_error(KnownError::Server)),
            QueryMethod::Put(_) => Err(response_base.into_error(KnownError::MethodUnknown)),
            QueryMethod::Get => Err(response_base.into_error(KnownError::MethodUnknown)),
        }
    }
}
