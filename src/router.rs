use crate::{node::Node, node_info::NodeInfo, u160::U160};
use bucket::Bucket;
use simple_error::{SimpleError, SimpleResult};
use std::{collections::{HashSet, VecDeque}, path::PathBuf};
use tokio::sync::{Mutex, RwLock};
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
            .unwrap_or_else(|e| Bucket::root(Router::generate_own_id(), K_SIZE));
        Ok(Self { buckets, banned_ids: RwLock::new(VecDeque::with_capacity(BAN_COUNT)) })
    }

    pub fn own_id(&self) -> U160 {
        self.buckets.id()
    }

    fn generate_own_id() -> U160 {
        //update later to secure version
        U160::rand()
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
        self.buckets.remove(id);
    }
}
