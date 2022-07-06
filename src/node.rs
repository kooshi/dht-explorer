use crate::messenger::message::kmsg::socket_addr_wrapper::SocketAddrWrapper;
use crate::messenger::message::{self, KnownError, MessageBase, Query, QueryResult, Receiver, ResponseKind, Sender};
use crate::messenger::{self, Messenger, QueryHandler, WrappedQueryHandler};
use crate::node_info::NodeInfo;
use crate::router::Router;
use crate::u160::U160;
use crate::utils::UnboundedConcurrentTaskSet;
mod token;
use self::token::TokenGenerator;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use messenger::message::QueryMethod;
use simple_error::{try_with, SimpleResult};
use sled::{self, Db};
use std::collections::{BTreeSet, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const PEER_TREE: &str = "peers";

//todo make config struct that builds node
pub struct Config {}

#[derive(Clone)]
pub struct Node {
    messenger: Arc<Messenger>,
    server:    Arc<Server>,
}

struct Server {
    router:      Router,
    transaction: AtomicUsize,
    read_only:   bool,
    me:          NodeInfo,
    db:          Db,
    tokens:      TokenGenerator,
}

impl Node {
    pub async fn new(addr: SocketAddr, read_only: bool, public_ip: IpAddr, state_dir: PathBuf) -> SimpleResult<Self> {
        let db = try_with!(
            sled::Config::new()
                .path(state_dir.join("sled"))
                .cache_capacity(1_000_000_000)
                .flush_every_ms(Some(5000))
                .open(),
            "open db"
        );
        //todo get public address from bootstrap
        let me = NodeInfo::from_addr(SocketAddr::new(public_ip, addr.port()));
        let router = Router::new(state_dir.join("buckets.ben"), me).await?;
        let tokens = TokenGenerator::new();
        let server = Arc::new(Server { router, transaction: AtomicUsize::new(0), read_only, me, db, tokens });
        let handler: Option<WrappedQueryHandler> = if read_only { None } else { Some(server.clone()) };
        let messenger = Arc::new(Messenger::new(addr, 500, handler, crate::MAX_CONCURRENCY).await?);
        Ok(Node { server, messenger })
    }

    pub async fn bootstrap(&self, bootstrap_node: SocketAddr) -> SimpleResult<()> {
        let response = try_with!(
            self.messenger.query(&self.build_query(Receiver::Addr(bootstrap_node), QueryMethod::Ping)).await,
            "could not reach bootstrap node"
        );
        self.server.router.add(response.origin.into()).await;
        let found = self.find(self.server.router.own_id(), false).await;
        info!("Bootstrapped. Found {found:?}");
        info!("Bucket stats: {}", self.server.router.stats().await);
        Ok(())
    }

    pub async fn find(&self, target: U160, find_peers: bool) -> Found {
        let mut tasks = UnboundedConcurrentTaskSet::new();
        let state = self.server.clone();
        tasks.add_task(async move { OneResult::FoundSome(state.router.lookup(target).await) });

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
        let mut peers = HashSet::new();
        while let Some(one_result) = tasks.get_next_result().await {
            match one_result {
                OneResult::FoundSome(found) =>
                    for found in found {
                        if found.id == target {
                            return Found::Target(found);
                        }
                        if seen.insert(Close(target.distance(found.id), found))
                            && seen.iter().position(|c| c.1.id == found.id).unwrap() < (crate::K_SIZE as usize * 2)
                        {
                            let selfclone = self.clone();
                            tasks.add_task(async move { selfclone.send_find(found, target, find_peers).await })
                        }
                    },
                OneResult::RemoveOne(n) => {
                    debug!("Ignoring node that didn't respond {n}");
                    ignore.insert(n);
                }
                OneResult::Peers(mut p) => p.drain(..).for_each(|p| {
                    peers.insert(p);
                }),
            }
        }
        if !peers.is_empty() {
            Found::Peers(peers)
        } else {
            Found::KClosest(
                seen.iter()
                    .filter_map(|n| if ignore.contains(&n.1) { None } else { Some(n.1) })
                    .take(crate::K_SIZE as usize)
                    .collect(),
            )
        }
    }

    async fn send_find(&self, to: NodeInfo, target: U160, find_peers: bool) -> OneResult {
        let method = if find_peers { QueryMethod::GetPeers(target) } else { QueryMethod::FindNode(target) };
        match self.messenger.query(&self.build_query(to.into(), method)).await {
            Ok(r) => {
                self.server.router.add(r.origin.into()).await;
                match r.kind {
                    ResponseKind::KNearest { nodes: n, .. } => OneResult::FoundSome(n),
                    ResponseKind::Peers { peers: p, .. } if find_peers => OneResult::Peers(p),
                    _ => {
                        warn!("unexpected find node response");
                        OneResult::RemoveOne(to)
                    }
                }
            }
            Err(e) => {
                error!("Received error response: {}", e);
                self.server.router.ban_id(to.id).await;
                OneResult::RemoveOne(to)
            }
        }
    }

    fn build_query(&self, to: Receiver, method: QueryMethod) -> Query {
        MessageBase::builder()
            .transaction_id(self.server.transaction.fetch_add(1, Ordering::Relaxed) as u16)
            .destination(to)
            .origin(Sender::Me(self.server.me))
            .read_only(self.server.read_only)
            .requestor_addr(Some(self.server.me.addr))
            .build()
            .into_query(method)
    }
}

enum OneResult {
    FoundSome(Vec<NodeInfo>),
    RemoveOne(NodeInfo),
    Peers(Vec<SocketAddr>),
}

#[derive(Debug)]
pub enum Found {
    Target(NodeInfo),
    KClosest(Vec<NodeInfo>),
    Peers(HashSet<SocketAddr>),
}

impl Server {
    fn response_base(&self, tid: u16, to: Receiver) -> MessageBase {
        MessageBase::builder()
            .origin(Sender::Me(self.me))
            .transaction_id(tid)
            .destination(to)
            .requestor_addr(to.into())
            .build()
    }

    fn handle_announce(
        &self, response_base: MessageBase, origin: NodeInfo, info_hash: U160, port: u16, token: &[u8],
    ) -> QueryResult {
        if !self.tokens.validate(token, origin.addr.ip()) {
            return Err(response_base.into_error(KnownError::Protocol));
        }
        if !origin.validate() {
            return Err(response_base.into_error(KnownError::InvalidNodeId));
        }
        let base_clone = response_base.clone();
        let peers = self.db.open_tree(PEER_TREE).map_err(|_| base_clone.into_error(KnownError::Server))?;
        let base_clone = response_base.clone();
        peers
            .fetch_and_update(info_hash.to_be_bytes(), |p| {
                let wrap = SocketAddrWrapper { socket_addr: Some(SocketAddr::new(origin.addr.ip(), port)) };
                let mut socks: Vec<SocketAddrWrapper> = p.map_or_else(Vec::new, |p| bt_bencode::from_slice(p).unwrap());
                if !socks.contains(&wrap) {
                    socks.push(wrap);
                }
                Some(bt_bencode::to_vec(&socks).unwrap())
            })
            .map_err(|_| base_clone.into_error(KnownError::Server))?;
        Ok(response_base.into_response(ResponseKind::Ok))
    }

    async fn handle_get_peers(&self, response_base: MessageBase, origin: NodeInfo, info_hash: U160) -> QueryResult {
        let token = self.tokens.generate(origin.addr.ip()).to_vec();
        let base_clone = response_base.clone();
        let base_clone2 = response_base.clone();
        let peers = self.db.open_tree(PEER_TREE).map_err(|_| base_clone.into_error(KnownError::Server))?;
        let peers = peers.get(info_hash.to_be_bytes()).map_err(|_| base_clone2.into_error(KnownError::Server))?;
        if let Some(peers) = peers {
            Ok(response_base.into_response(ResponseKind::Peers {
                peers: bt_bencode::from_slice::<Vec<SocketAddrWrapper>>(&peers)
                    .unwrap()
                    .iter()
                    .map(|a| a.socket_addr.unwrap())
                    .collect(),
                token,
            }))
        } else {
            Ok(response_base.into_response(ResponseKind::KNearest {
                nodes: self.router.lookup(info_hash).await,
                token: Some(token),
            }))
        }
    }
}

#[async_trait]
impl QueryHandler for Server {
    async fn handle_query(&self, query: Query) -> QueryResult {
        assert!(!self.read_only);
        let response_base = self.response_base(query.transaction_id, query.origin.into());
        if query.origin.id() == self.me.id {
            return Err(response_base.into_error_generic("Echo!"));
        }
        if !query.read_only {
            self.router.add(query.origin.into()).await;
        }
        match &query.method {
            QueryMethod::Ping => Ok(response_base.into_response(ResponseKind::Ok)),
            QueryMethod::FindNode(n) => Ok(response_base
                .into_response(ResponseKind::KNearest { nodes: self.router.lookup(*n).await, token: None })),
            QueryMethod::GetPeers(info_hash) =>
                self.handle_get_peers(response_base, query.origin.into(), *info_hash).await,
            QueryMethod::AnnouncePeer { info_hash, token, port } =>
                self.handle_announce(response_base, query.origin.into(), *info_hash, *port, token),
            QueryMethod::Put(_) => Err(response_base.into_error(KnownError::MethodUnknown)),
            QueryMethod::Get => Err(response_base.into_error(KnownError::MethodUnknown)),
        }
    }

    async fn handle_error(&self, tid: u16, source_addr: SocketAddr) -> message::Error {
        self.response_base(tid, Receiver::Addr(source_addr)).into_error(message::KnownError::Server)
    }
}
