use crate::{node_info::NodeInfo, param, u160::U160};
use bucket::Bucket;
use log::{debug, info};
use simple_error::SimpleResult;
use std::{collections::VecDeque, net::IpAddr, path::PathBuf, str::FromStr};
use tokio::sync::RwLock;
mod bucket;

pub struct Router {
    buckets:    Bucket,
    banned_ids: RwLock<VecDeque<U160>>,
}
const BAN_COUNT: usize = 100;
const K_SIZE: u8 = 10;
impl Router {
    pub async fn new(bucket_file: PathBuf) -> SimpleResult<Self> {
        let buckets = Bucket::load_from_file(bucket_file)
            .await
            .unwrap_or_else(|_| Bucket::root(Router::generate_own_id(), K_SIZE));
        Ok(Self { buckets, banned_ids: RwLock::new(VecDeque::with_capacity(BAN_COUNT)) })
    }

    pub fn own_id(&self) -> U160 {
        self.buckets.id()
    }

    fn generate_own_id() -> U160 {
        param!()
            .public_ip
            .as_ref()
            .map_or(None, |ip| IpAddr::from_str(&ip).ok())
            .map(|ip| U160::from_ip(&ip))
            .unwrap_or(U160::rand())
    }

    pub async fn add(&self, node: NodeInfo) {
        if !self.banned_ids.read().await.contains(&node.id) {
            self.buckets.add(node).await
        }
    }

    pub async fn lookup(&self, id: U160) -> Vec<NodeInfo> {
        self.buckets.lookup(id).await
    }

    pub async fn ban_id(&self, id: U160) {
        {
            let mut bnd = self.banned_ids.write().await;
            bnd.push_back(id);
            if bnd.len() == BAN_COUNT {
                bnd.pop_front();
            }
        }
        debug!("Banned id {}", id);
        if !self.buckets.remove(id).await {
            info!("Failed to remove {}", id)
        }
    }
}
