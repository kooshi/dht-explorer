use crate::{messenger::{self, message::{MessageBase, Query, QueryResult, ResponseKind}, Messenger, QueryHandler, WrappedQueryHandler}, node_info::NodeInfo, router::Router, u160::U160};
use async_trait::async_trait;
use log::error;
use messenger::message::QueryMethod;
use rand::{seq::SliceRandom, thread_rng};
use simple_error::SimpleResult;
use std::{net::SocketAddr, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

pub struct Node {
    messenger: Messenger,
    state:     Arc<NodeState>,
}

impl Node {
    pub async fn new(addr: SocketAddr, read_only: bool) -> SimpleResult<Self> {
        let router = Router::new("./target/buckets.ben".into()).await?;
        let state = Arc::new(NodeState { router, transaction: AtomicUsize::new(0), read_only });
        let handler: Option<WrappedQueryHandler> = if read_only { None } else { Some(state.clone()) };
        let messenger = Messenger::new(addr, 500, handler).await?;
        Ok(Node { messenger, state })
    }

    pub async fn bootstrap(&self, bootstrap_node: SocketAddr) -> SimpleResult<()> {
        self.send_find_node(bootstrap_node, self.state.router.own_id()).await;
        while let None = self.find_node(self.state.router.own_id()).await {
            //self.find_node(U160::rand()).await;
        }
        Ok(())
    }

    pub async fn find_node(&self, id: U160) -> Option<NodeInfo> {
        let closest_known = self.state.router.lookup(id).await; //.choose(&mut rand::thread_rng());
        if let Some(target) = closest_known.iter().find(|n| n.id == id) {
            return Some(target.clone());
        }
        self.send_find_node(closest_known.choose(&mut thread_rng()).unwrap().addr, id).await;
        None
    }

    async fn send_find_node(&self, to: SocketAddr, id: U160) {
        match self.messenger.query(&self.message_base(to).to_query(QueryMethod::FindNode(id))).await {
            Ok(r) => {
                self.state.router.add(NodeInfo { addr: r.received_from_addr.unwrap(), id: r.sender_id }).await;
                if let ResponseKind::KNearest(nodes) = r.kind {
                    for n in nodes {
                        self.state.router.add(n).await;
                    }
                }
            }
            Err(e) => {
                self.state.router.ban_id(id).await;
                error!("Received error response: {}", e)
            }
        }
    }

    fn message_base(&self, to: SocketAddr) -> MessageBase {
        MessageBase::builder()
            .sender_id(self.state.router.own_id())
            .transaction_id(hex::encode(self.state.transaction.fetch_add(1, Ordering::Relaxed).to_be_bytes()))
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
            let node = NodeInfo { id: query.sender_id, addr: query.received_from_addr.unwrap() };
            self.router.add(node).await;
        }
        let response_base = MessageBase::builder()
            .sender_id(self.router.own_id())
            .transaction_id(query.transaction_id.clone())
            .destination_addr(query.received_from_addr.unwrap())
            .build();
        match query.method {
            QueryMethod::Ping => Ok(response_base.to_response(ResponseKind::Ok)),
            QueryMethod::FindNode(_) => todo!(),
            QueryMethod::GetPeers(_) => todo!(),
            QueryMethod::AnnouncePeer(_) => todo!(),
            QueryMethod::Put(_) => todo!(),
            QueryMethod::Get => todo!(),
        }
    }
}
