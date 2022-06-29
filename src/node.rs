use crate::{messenger::{self, message::{KnownError, MessageBase, Query, QueryResult, ResponseKind}, Messenger, QueryHandler, WrappedQueryHandler}, node_info::NodeInfo, router::Router, u160::U160};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use messenger::message::QueryMethod;
use simple_error::{try_with, SimpleResult};
use std::{collections::{BTreeSet, HashSet}, net::{IpAddr, SocketAddr}, str::FromStr, sync::{atomic::{AtomicUsize, Ordering}, Arc}};
use tokio::sync::mpsc::{self, UnboundedSender};

#[derive(Clone)]
pub struct Node {
    messenger: Arc<Messenger>,
    state:     Arc<NodeState>,
}

impl Node {
    pub async fn new(addr: SocketAddr, read_only: bool, public_ip: IpAddr) -> SimpleResult<Self> {
        let router = Router::new("./target/buckets.ben".into(), public_ip).await?;
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
        let found = self.find_node(self.state.router.own_id()).await;
        info!("Bootstrapped. Found {found:?}");
        info!("Bucket stats: {}", self.state.router.stats().await);
        Ok(())
    }

    pub async fn find_node(&self, target: U160) -> Found {
        let (tx, mut rx) = mpsc::unbounded_channel();
        for known in self.state.router.lookup(target).await {
            tx.send(OneResult::FoundOne(known, tx.clone())).unwrap_or_else(|_| error!("init find node"));
        }
        drop(tx);
        #[derive(PartialEq, Eq)]
        struct Close(U160, NodeInfo);
        impl PartialOrd for Close {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&other.0)
            }
        }
        impl Ord for Close {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }
        let mut ignore = HashSet::new();
        let mut seen = BTreeSet::new();
        while let Some(one_result) = rx.recv().await {
            match one_result {
                OneResult::FoundOne(found, tx) => {
                    if found.id == target {
                        return Found::Target(found);
                    }
                    if seen.insert(Close(target.distance(found.id), found))
                        && seen.iter().position(|c| c.1.id == found.id).unwrap() < (crate::K_SIZE as usize * 2)
                    {
                        let selfclone = self.clone();
                        let txclone = tx.clone();
                        tokio::spawn(async move {
                            selfclone.send_find_node(found, target, txclone).await;
                        });
                    }
                }
                OneResult::RemoveOne(n) => {
                    debug!("Ignoring node that didn't respond {n}");
                    ignore.insert(n);
                }
            }
        }
        Found::KClosest(
            seen.iter()
                .filter_map(|n| if ignore.contains(&n.1) { None } else { Some(n.1) })
                .take(crate::K_SIZE as usize)
                .collect(),
        )
    }

    async fn send_find_node(&self, to: NodeInfo, id: U160, found: UnboundedSender<OneResult>) {
        match self.messenger.query(&self.message_base(to.addr).into_query(QueryMethod::FindNode(id))).await {
            Ok(r) => {
                self.state.router.add(r.origin).await;
                if let ResponseKind::KNearest(nodes) = r.kind {
                    for n in nodes {
                        self.state.router.add(n).await;
                        found.send(OneResult::FoundOne(n, found.clone())).unwrap_or_else(|_| warn!("return find node"));
                    }
                } else {
                    warn!("unexpected find node response")
                }
            }
            Err(e) => {
                error!("Received error response: {}", e);
                self.state.router.ban_id(to.id).await;
                found.send(OneResult::RemoveOne(to)).unwrap_or_else(|_| warn!("return find node"));
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
pub enum OneResult {
    FoundOne(NodeInfo, UnboundedSender<OneResult>),
    RemoveOne(NodeInfo),
}
#[derive(Debug)]
pub enum Found {
    Target(NodeInfo),
    KClosest(Vec<NodeInfo>),
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
