use crate::{node_info::NodeInfo, u160::U160};
use async_recursion::async_recursion;
use log::{debug, info, trace};
use serde::{self, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use simple_error::{try_with, SimpleResult};
use std::{fs::OpenOptions, ops::Deref, path::PathBuf, sync::Arc};
use tokio::sync::{OnceCell, RwLock};

const MAX_BUCKET_INDEX: u8 = 159;

#[derive(Debug)]
pub struct Bucket {
    own_id:       U160,
    bucket_index: u8,
    k_size:       u8,
    nodes:        RwLock<Vec<NodeInfo>>,
    next_bucket:  OnceCell<Box<Bucket>>,
}

impl Bucket {
    pub fn root(own_id: U160, k_size: u8) -> Self {
        Self {
            own_id,
            k_size,
            nodes: RwLock::new(Vec::with_capacity(k_size as usize)),
            next_bucket: OnceCell::new(),
            bucket_index: 0,
        }
    }

    pub fn id(&self) -> U160 {
        self.own_id
    }

    #[async_recursion]
    pub async fn add(&self, node: NodeInfo) {
        if self.next_bucket.get().is_none() && self.nodes.read().await.len() < self.k_size.into() {
            self.add_or_update(node).await;
            return;
        }
        if self.next_bucket.get().is_none() && self.nodes.read().await.len() == self.k_size.into() {
            if !self.make_next_bucket().await {
                return;
            }
        }
        if !self.belongs_here(&node) {
            self.next_bucket.get().unwrap().add(node).await;
            return;
        }
        self.add_or_update(node).await;
    }

    async fn add_or_update(&self, node: NodeInfo) {
        let mut nodes = self.nodes.write().await;
        if let Some(index) = nodes.iter().rev().position(|&n| n.id == node.id) {
            let mut index = nodes.len() - 1 - index; //inverted position
            while index < nodes.len() - 1 {
                nodes.swap(index, index + 1);
                index += 1;
            }
            trace!("Bumped {} in bucket {}", node, self.bucket_index);
        } else if nodes.len() < self.k_size.into() {
            nodes.push(node);
            trace!("Stored {} in bucket {}", node, self.bucket_index);
        }
    }

    #[async_recursion]
    pub async fn lookup(&self, id: U160) -> Vec<NodeInfo> {
        let mut k_nearest = if self.next_bucket.get().is_some() && !self.id_belongs_here(id) {
            self.next_bucket.get().unwrap().lookup(id).await
        } else {
            Vec::with_capacity(self.k_size.into())
        };

        let gap = self.k_size as usize - k_nearest.len();
        if gap > 0 {
            k_nearest.extend(self.nodes.read().await.iter().take(gap).cloned().collect::<Vec<_>>());
        }

        let gap = self.k_size as usize - k_nearest.len();
        if gap > 0 && self.next_bucket.get().is_some() {
            k_nearest
                .extend(self.next_bucket.get().unwrap().lookup(id).await.iter().take(gap).cloned().collect::<Vec<_>>());
        }

        k_nearest
    }

    #[async_recursion]
    pub async fn remove(&self, id: U160) -> bool {
        if self.next_bucket.get().is_none() || self.id_belongs_here(id) {
            let mut nodes = self.nodes.write().await;
            if let Some(i) = nodes.iter().position(|n| n.id == id) {
                let node = nodes.remove(i);
                debug!("Removed {} from bucket {}", node, self.bucket_index);
                true
            } else {
                false
            }
        } else {
            self.next_bucket.get().unwrap().remove(id).await
        }
    }

    #[async_recursion]
    async fn make_next_bucket(&self) -> bool {
        if self.bucket_index == MAX_BUCKET_INDEX {
            return false;
        }

        let set = Box::new(Bucket {
            own_id:       self.own_id,
            k_size:       self.k_size,
            nodes:        RwLock::new(Vec::with_capacity(self.k_size as usize)),
            next_bucket:  OnceCell::new(),
            bucket_index: self.bucket_index + 1,
        });

        if self.next_bucket.set(set).is_err() {
            return true; //already set, race condition got us here
        }

        {
            let mut nodes = self.nodes.write().await;
            let mut index = 0;
            while index < nodes.len() {
                if self.belongs_here(&nodes[index]) {
                    index += 1;
                } else {
                    self.next_bucket.get().unwrap().add(nodes.swap_remove(index)).await;
                }
            }
        }
        debug!("Created bucket {}", self.bucket_index + 1);
        true
    }

    fn belongs_here(&self, node: &NodeInfo) -> bool {
        self.own_id.distance(node.id).get_bit(self.bucket_index)
    }

    fn id_belongs_here(&self, id: U160) -> bool {
        self.own_id.distance(id).get_bit(self.bucket_index)
    }

    pub async fn save_to_file(bucket: Arc<Bucket>, path: PathBuf) -> SimpleResult<()> {
        let file = try_with!(OpenOptions::new().write(true).truncate(true).create(true).open(path), "opening file");
        let copy = bucket.clone();
        try_with!(
            try_with!(
                tokio::task::spawn_blocking(move || bt_bencode::to_writer(file, copy.as_ref())).await,
                "serialize"
            ),
            "join"
        );
        info!("Saved {}", bucket.stats().await);
        Ok(())
    }

    pub async fn load_from_file(path: PathBuf) -> SimpleResult<Self> {
        let file = try_with!(OpenOptions::new().read(true).open(path), "opening file");
        let bucket: Self = try_with!(bt_bencode::from_reader(file), "deser");
        info!("Loaded {}", bucket.stats().await);
        Ok(bucket)
    }

    pub async fn stats(&self) -> String {
        let mut stats = Vec::with_capacity(50);
        stats.push(self.nodes.read().await.len());
        let mut next = self.next_bucket.get();
        while next.is_some() {
            let bucket = next.unwrap();
            stats.push(bucket.nodes.read().await.len());
            next = bucket.next_bucket.get();
        }
        format!("{} total nodes in {} buckets", stats.iter().sum::<usize>(), stats.len())
    }
}

//TODO improve later so spawn_blocking doesn't eat the bucket when we save
impl Serialize for Bucket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut s = serializer.serialize_seq(Some(2))?;
        s.serialize_element(&self.own_id)?;
        s.serialize_element(&self.k_size)?;
        ser_df(&self, &mut s, 1)?;
        s.end()
    }
}

fn ser_df<S>(b: &Bucket, s: &mut S, depth: u16) -> Result<(), S::Error>
where S: SerializeSeq {
    if let Some(next) = &b.next_bucket.get() {
        ser_df(next, s, depth + 1)?;
    } else {
        s.serialize_element(&depth)?;
    }
    s.serialize_element(&*b.nodes.blocking_read())
}

impl<'de> Deserialize<'de> for Bucket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        struct Visitor {}
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Bucket;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!("expected error code followed by message"))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where A: serde::de::SeqAccess<'de> {
                let own_id = seq.next_element()?.unwrap();
                let k_size = seq.next_element()?.unwrap();
                let mut bucket_index = seq.next_element()?.unwrap();
                let mut deser = Vec::with_capacity(bucket_index as usize + 1);
                while let Some(nodes) = seq.next_element::<Vec<NodeInfo>>()? {
                    bucket_index -= 1;
                    deser.push(Bucket {
                        own_id,
                        k_size,
                        bucket_index,
                        nodes: RwLock::new(nodes),
                        next_bucket: OnceCell::new(),
                    });
                }
                Ok(deser
                    .drain(..)
                    .reduce(|child, parent| {
                        parent.next_bucket.set(Box::new(child)).unwrap();
                        parent
                    })
                    .unwrap())
            }
        }
        deserializer.deserialize_seq(Visitor {})
    }
}

impl PartialEq for Bucket {
    fn eq(&self, other: &Self) -> bool {
        self.own_id == other.own_id
            && self.bucket_index == other.bucket_index
            && self.k_size == other.k_size
            && self.nodes.blocking_read().deref() == other.nodes.blocking_read().deref()
            && self.next_bucket == other.next_bucket
    }
}

#[cfg(test)]
mod tests {
    use super::Bucket;
    use crate::{node_info::NodeInfo, u160::U160, utils};
    use log::info;
    use std::{net::SocketAddr, ops::Deref, str::FromStr, sync::Arc};

    #[tokio::test]
    async fn full() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let bucket = Arc::new(Bucket::root(U160::empty(), 8));
        let test_node = NodeInfo { id: U160::from_hex("ffffffffffffffffffffffffffffffffffffffff"), addr: socket };
        bucket.add(test_node).await;
        fill(bucket.clone()).await;
        bucket.add(test_node).await; //update
        info!("{}", bucket.stats().await);
        assert_eq!(bucket.nodes.write().await.pop().unwrap().id, test_node.id);
    }

    #[tokio::test]
    async fn remove() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let bucket = Arc::new(Bucket::root(U160::empty(), 8));
        let test_node = NodeInfo { id: U160::from_hex("ffffffffffffffffffffffffffffffffffffffff"), addr: socket };
        bucket.add(test_node).await;
        fill(bucket.clone()).await;
        bucket.add(test_node).await; //update
        bucket.remove(test_node.id).await;
        println!("{:?}", bucket);
        assert_ne!(bucket.nodes.write().await.pop().unwrap().id, test_node.id);
    }

    async fn fill(bucket: Arc<Bucket>) {
        let mut joins = Vec::with_capacity(10);
        for _ in 0..10 {
            let bucket = bucket.clone();
            joins.push(tokio::spawn(async move {
                for _ in 0..200 {
                    bucket
                        .add(NodeInfo {
                            id:   U160::rand() >> (rand::random::<u8>() % 161),
                            addr: SocketAddr::from_str("127.0.0.1:1337").unwrap(),
                        })
                        .await
                }
            }))
        }
        futures::future::join_all(joins).await;
    }

    #[tokio::test]
    async fn lookup() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let bucket = Arc::new(Bucket::root(U160::empty(), 30));
        fill(bucket.clone()).await;
        for _ in 0..60 {
            bucket.add(NodeInfo { id: U160::rand() >> (rand::random::<u8>() % 161), addr: socket }).await
        }

        let query = U160::rand() >> (rand::random::<u8>() % 161);
        let k_nearest = bucket.lookup(query).await;
        println!("Searching for: {:?}\nFound:\n{:?}", query, k_nearest);

        let query = U160::rand();
        let k_nearest = bucket.lookup(query).await;
        println!("Searching for: {:?}\nFound:\n{:?}", query, k_nearest);

        let k_nearest = bucket.lookup(U160::empty()).await;
        println!("Searching for: {:?}\nFound:\n{:?}", U160::empty(), k_nearest);
    }

    #[tokio::test]
    async fn fill_and_lookup_concurrent() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let bucket = Arc::new(Bucket::root(U160::empty(), 8));
        let test_node = NodeInfo { id: U160::from_hex("ffffffffffffffffffffffffffffffffffffffff"), addr: socket };
        bucket.add(test_node).await;

        let mut joins = Vec::with_capacity(20);
        for _ in 0..10 {
            let bucket = bucket.clone();
            joins.push(tokio::spawn(async move {
                for _ in 0..10_000 {
                    bucket
                        .add(NodeInfo {
                            id:   U160::rand() >> (rand::random::<u8>() % 161),
                            addr: SocketAddr::from_str("127.0.0.1:1337").unwrap(),
                        })
                        .await
                }
            }))
        }
        for _ in 0..10 {
            let bucket = bucket.clone();
            joins.push(tokio::spawn(async move {
                for _ in 0..10_000 {
                    let query = U160::rand() >> (rand::random::<u8>() % 161);
                    let _k_nearest = bucket.lookup(query).await;
                }
            }))
        }
        futures::future::join_all(joins).await;

        bucket.add(test_node).await; //update
        println!("{:?}", bucket);
        assert_eq!(bucket.nodes.write().await.pop().unwrap().id, test_node.id);
    }

    #[tokio::test]
    async fn serde() {
        let bucket = Arc::new(Bucket::root(U160::empty(), 8));
        fill(bucket.clone()).await;
        let serbuck = bucket.clone();
        let ser = tokio::task::spawn_blocking(move || bt_bencode::to_vec(serbuck.as_ref()).unwrap()).await.unwrap();
        println!("SER: {}", utils::safe_string_from_slice(&ser));
        let de: Bucket = tokio::task::spawn_blocking(move || bt_bencode::from_slice(&ser).unwrap()).await.unwrap();
        assert!(tokio::task::spawn_blocking(move || &de == bucket.deref()).await.unwrap());
    }

    #[tokio::test]
    async fn file() {
        let bucket = Arc::new(Bucket::root(U160::empty(), 8));
        fill(bucket.clone()).await;
        let path = "./target/bucket_test.ben";
        Bucket::save_to_file(bucket.clone(), path.into()).await.unwrap();
        let b2 = Bucket::load_from_file(path.into()).await.unwrap();
        tokio::fs::remove_file(path).await.unwrap();

        assert!(tokio::task::spawn_blocking(move || &b2 == bucket.deref()).await.unwrap());
    }
}
