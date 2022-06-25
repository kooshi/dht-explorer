use crate::{bucket::Bucket, messenger::{self, message::{MessageBase, Query, QueryResult, ResponseKind}, Messenger, QueryHandler, WrappedQueryHandler}, node_info::NodeInfo, u160::U160};
use async_trait::async_trait;
use messenger::message::QueryMethod;
use rand::{seq::SliceRandom, thread_rng};
use simple_error::SimpleResult;
use std::{net::SocketAddr, sync::{atomic::{AtomicUsize, Ordering}, Arc}};
use tokio::sync::Mutex;
pub struct Node {
    messenger: Messenger,
    state:     Arc<NodeState>,
}

impl Node {
    pub async fn new(addr: SocketAddr, bucket: Option<Bucket>, read_only: bool) -> SimpleResult<Self> {
        let bucket = if let Some(bucket) = bucket { bucket } else { Bucket::root(U160::rand(), 8) };
        let info = NodeInfo { addr, id: bucket.id() };
        let state =
            Arc::new(NodeState { info, bucket: Mutex::new(bucket), transaction: AtomicUsize::new(0), read_only });
        let handler: Option<WrappedQueryHandler> = if read_only { None } else { Some(state.clone()) };
        let messenger = Messenger::new(addr, 500, handler).await?;
        Ok(Node { messenger, state })
    }

    pub async fn bootstrap(&self, bootstrap_node: SocketAddr) -> SimpleResult<()> {
        self.send_find_node(bootstrap_node, self.state.info.id).await;
        while let None = self.find_node(self.state.info.id).await {
            //self.find_node(U160::rand()).await;
        }
        self.state.bucket.lock().await.save_to_file("./found_nodes.ben".into()).await?;
        Ok(())
    }

    pub async fn find_node(&self, id: U160) -> Option<NodeInfo> {
        let closest_known = self.state.bucket.lock().await.lookup(id); //.choose(&mut rand::thread_rng());
        if let Some(target) = closest_known.iter().find(|n| n.id == id) {
            return Some(target.clone());
        }
        self.send_find_node(closest_known.choose(&mut thread_rng()).unwrap().addr, id).await;
        None
    }

    async fn send_find_node(&self, to: SocketAddr, id: U160) {
        match self.messenger.query(&self.message_base(to).to_query(QueryMethod::FindNode(id))).await {
            Ok(r) => {
                self.state.bucket.lock().await.add(NodeInfo { addr: r.received_from_addr.unwrap(), id: r.sender_id });
                if let ResponseKind::KNearest(nodes) = r.kind {
                    let mut bucket = self.state.bucket.lock().await;
                    for n in nodes {
                        bucket.add(n);
                    }
                }
            }
            Err(e) => (),
        }
    }

    fn message_base(&self, to: SocketAddr) -> MessageBase {
        MessageBase::builder()
            .sender_id(self.state.info.id)
            .transaction_id(hex::encode(self.state.transaction.fetch_add(1, Ordering::Relaxed).to_be_bytes()))
            .destination_addr(to)
            .read_only(self.state.read_only)
            .build()
    }
}

pub struct NodeState {
    info:        NodeInfo,
    bucket:      Mutex<Bucket>,
    transaction: AtomicUsize,
    read_only:   bool,
}

impl NodeState {
    async fn store_node(&self, node: NodeInfo) {
        let mut b = self.bucket.lock().await;
        b.add(node);
    }
}

#[async_trait]
impl QueryHandler for NodeState {
    async fn handle_query(&self, query: Query) -> QueryResult {
        assert!(!self.read_only);
        if !query.read_only {
            let node = NodeInfo { id: query.sender_id, addr: query.received_from_addr.unwrap() };
            self.store_node(node).await;
        }
        let response_base = MessageBase::builder()
            .sender_id(self.info.id)
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
