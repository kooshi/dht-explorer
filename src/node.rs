use crate::{messenger::{self, message::{KnownError, MessageBase, Query, QueryResult, ResponseKind}, Messenger, QueryHandler, WrappedQueryHandler}, node_info::NodeInfo, router::Router, u160::U160};
use async_trait::async_trait;
use log::{error, warn};
use messenger::message::QueryMethod;
use simple_error::{try_with, SimpleResult};
use std::{collections::BTreeSet, net::SocketAddr, str::FromStr, sync::{atomic::{AtomicUsize, Ordering}, Arc}};
use tokio::sync::mpsc::{self, UnboundedSender};

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
        let messenger = Arc::new(Messenger::new(addr, 500, handler, crate::MAX_CONCURRENCY).await?);
        Ok(Node { messenger, state })
    }

    pub async fn bootstrap(&self, bootstrap_node: SocketAddr) -> SimpleResult<()> {
        let response = try_with!(
            self.messenger.query(&self.message_base(bootstrap_node).into_query(QueryMethod::Ping)).await,
            "could not reach bootstrap node"
        );
        self.state.router.add(response.origin).await;
        self.find_node(self.state.router.own_id()).await;
        Ok(())
    }

    pub async fn find_node(&self, target: U160) -> Option<NodeInfo> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        for known in self.state.router.lookup(target).await {
            tx.send(Found(known, tx.clone())).unwrap_or_else(|_| error!("init find node"));
        }
        drop(tx);
        #[derive(PartialEq, Eq)]
        struct Close<'a>(&'a U160, NodeInfo);
        impl PartialOrd for Close<'_> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.0.distance(self.1.id).partial_cmp(&other.0.distance(other.1.id))
            }
        }
        impl Ord for Close<'_> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.distance(self.1.id).cmp(&other.0.distance(other.1.id))
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(Found(found, tx)) = rx.recv().await {
            if found.id == target {
                return Some(found);
            }
            if seen.insert(Close(&target, found))
                && seen.iter().position(|c| c.1.id == found.id).unwrap() < (crate::K_SIZE as usize * 2)
            {
                let selfclone = self.clone();
                let txclone = tx.clone();
                tokio::spawn(async move {
                    selfclone.send_find_node(found, target, txclone).await;
                });
            }
        }
        None
    }

    async fn send_find_node(&self, to: NodeInfo, id: U160, found: UnboundedSender<Found>) {
        match self.messenger.query(&self.message_base(to.addr).into_query(QueryMethod::FindNode(id))).await {
            Ok(r) => {
                self.state.router.add(r.origin).await;
                if let ResponseKind::KNearest(nodes) = r.kind {
                    for n in nodes {
                        self.state.router.add(n).await;
                        found.send(Found(n, found.clone())).unwrap_or_else(|_| warn!("return find node"));
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
struct Found(NodeInfo, UnboundedSender<Found>);

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
            .transaction_id(query.transaction_id)
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

#[cfg(test)]
mod test {
    use rand::random;
    //use std::collections::BTreeSet;
    use tokio::sync::mpsc::Sender;
    struct Rec(u8, Sender<Rec>);

    #[tokio::test]
    async fn channel() {
        use tokio::{sync::mpsc, task};
        //let seen = BTreeSet::new();
        let (tx, mut rx) = mpsc::channel(20);
        let mut seen = vec![false; 256];
        task::spawn(async move {
            for _ in 0..10 {
                tx.send(Rec(random(), tx.clone())).await.ok();
            }
        });
        while let Some(Rec(val, tx)) = rx.recv().await {
            if !seen[val as usize] {
                seen[val as usize] = true;
                task::spawn(async move {
                    for _ in 0..10 {
                        tx.send(Rec(random(), tx.clone())).await.ok();
                    }
                });
            }
        }
        println!("lol")
    }
}
