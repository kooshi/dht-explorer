use crate::node_info::NodeInfo;
use crate::u160::U160;
use bucket::Bucket;
use log::{debug, info};
use simple_error::SimpleResult;
use std::collections::VecDeque;
use std::path::PathBuf;
use tokio::sync::RwLock;
mod bucket;

pub struct Router {
    buckets:    Bucket,
    banned_ids: RwLock<VecDeque<U160>>,
}
const BAN_COUNT: usize = 100;
impl Router {
    pub async fn new(bucket_file: PathBuf, node: NodeInfo) -> SimpleResult<Self> {
        let buckets =
            Bucket::load_from_file(bucket_file).await.unwrap_or_else(|_| Bucket::root(node.id, crate::K_SIZE));
        Ok(Self { buckets, banned_ids: RwLock::new(VecDeque::with_capacity(BAN_COUNT)) })
    }

    pub fn own_id(&self) -> U160 {
        self.buckets.id()
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

    pub async fn stats(&self) -> String {
        self.buckets.stats().await
    }
}
