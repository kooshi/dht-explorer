use crate::{bucket::Bucket, messenger::{self, message::{MessageBase, Query, QueryResult, ResponseKind}, Messenger, QueryHandler, WrappedQueryHandler}, node_info::NodeInfo, u160::U160};
use async_trait::async_trait;
use messenger::message::QueryMethod;
use simple_error::SimpleResult;
use std::{net::SocketAddr, ops::Deref, path::PathBuf, sync::{atomic::{AtomicUsize, Ordering}, Arc}};
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

    pub async fn ping(&self, to: SocketAddr) -> QueryResult {
        self.messenger.query(&self.message_base(to).to_query(QueryMethod::Ping)).await
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

#[async_trait]
impl QueryHandler for NodeState {
    async fn handle_query(&self, query: Query) -> QueryResult {
        assert!(self.read_only);
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
