use crate::{node_info::NodeInfo, u160::U160};
use serde::{self, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use simple_error::{try_with, SimpleResult};
use std::path::PathBuf;
use tokio::fs::OpenOptions;

const MAX_BUCKET_INDEX: u8 = 159;

#[derive(Debug, PartialEq, Eq)]
pub struct Bucket {
    own_id:       U160,
    bucket_index: u8,
    k_size:       u8,
    nodes:        Vec<NodeInfo>,
    next_bucket:  Option<Box<Bucket>>,
}

impl Bucket {
    pub fn root(own_id: U160, k_size: u8) -> Self {
        Self { own_id, k_size, nodes: Vec::with_capacity(k_size as usize), next_bucket: None, bucket_index: 0 }
    }

    pub fn id(&self) -> U160 {
        self.own_id
    }

    pub fn add(&mut self, node: NodeInfo) {
        if self.next_bucket.is_none() && self.nodes.len() < self.k_size.into() {
            self.add_or_update(node);
            return;
        }
        if self.next_bucket.is_none() && self.nodes.len() == self.k_size.into() {
            if !self.make_next_bucket() {
                return;
            }
        }
        if !self.belongs_here(&node) {
            self.next_bucket.as_mut().unwrap().add(node);
            return;
        }
        self.add_or_update(node)
    }

    fn add_or_update(&mut self, node: NodeInfo) {
        if let Some(index) = self.nodes.iter().rev().position(|&n| n.id == node.id) {
            let mut index = self.nodes.len() - 1 - index; //inverted position
            while index < self.nodes.len() - 1 {
                self.nodes.swap(index, index + 1);
                index += 1;
            }
        } else if self.nodes.len() < self.k_size.into() {
            self.nodes.push(node);
        }
    }

    pub fn lookup(&self, id: U160) -> Vec<NodeInfo> {
        let mut k_nearest = if self.next_bucket.is_some() && !self.id_belongs_here(id) {
            self.next_bucket.as_ref().unwrap().lookup(id)
        } else {
            Vec::with_capacity(self.k_size.into())
        };

        let gap = self.k_size as usize - k_nearest.len();
        if gap > 0 {
            k_nearest.extend(self.nodes.iter().take(gap).cloned().collect::<Vec<_>>());
        }

        let gap = self.k_size as usize - k_nearest.len();
        if gap > 0 && self.next_bucket.is_some() {
            k_nearest
                .extend(self.next_bucket.as_ref().unwrap().lookup(id).iter().take(gap).cloned().collect::<Vec<_>>());
        }

        k_nearest
    }

    fn make_next_bucket(&mut self) -> bool {
        assert!(self.next_bucket.is_none());
        if self.bucket_index == MAX_BUCKET_INDEX {
            return false;
        }

        self.next_bucket = Some(Box::new(Bucket {
            own_id:       self.own_id,
            k_size:       self.k_size,
            nodes:        Vec::with_capacity(self.k_size as usize),
            next_bucket:  None,
            bucket_index: self.bucket_index + 1,
        }));

        let mut index = 0;
        while index < self.nodes.len() {
            if self.belongs_here(&self.nodes[index]) {
                index += 1;
            } else {
                self.next_bucket.as_mut().unwrap().add(self.nodes.swap_remove(index));
            }
        }
        true
    }

    fn belongs_here(&self, node: &NodeInfo) -> bool {
        self.own_id.distance(node.id).get_bit(self.bucket_index)
    }

    fn id_belongs_here(&self, id: U160) -> bool {
        self.own_id.distance(id).get_bit(self.bucket_index)
    }

    pub async fn save_to_file(&self, path: PathBuf) -> SimpleResult<()> {
        let file =
            try_with!(OpenOptions::new().write(true).truncate(true).create(true).open(path).await, "opening file");
        let file = file.into_std().await;
        try_with!(bt_bencode::to_writer(file, self), "serialize");
        Ok(())
    }

    pub async fn load_from_file(path: PathBuf) -> SimpleResult<Self> {
        let file = try_with!(OpenOptions::new().read(true).open(path).await, "opening file");
        let file = file.into_std().await;
        let bucket = try_with!(bt_bencode::from_reader(file), "deser");
        Ok(bucket)
    }
}

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
    if let Some(next) = &b.next_bucket {
        ser_df(next, s, depth + 1)?;
    } else {
        s.serialize_element(&depth)?;
    }
    s.serialize_element(&b.nodes)
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
                let mut last_bucket = None;
                while let Some(nodes) = seq.next_element::<Vec<NodeInfo>>()? {
                    bucket_index -= 1;
                    last_bucket =
                        Some(Box::new(Bucket { own_id, k_size, bucket_index, nodes, next_bucket: last_bucket }));
                }

                Ok(*last_bucket.unwrap())
            }
        }
        deserializer.deserialize_seq(Visitor {})
    }
}

#[cfg(test)]
mod tests {
    use super::Bucket;
    use crate::{node_info::NodeInfo, u160::U160, utils};
    use std::{net::SocketAddr, str::FromStr};

    #[test]
    fn full() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let mut bucket = Bucket::root(U160::empty(), 8);
        let test_node = NodeInfo { id: U160::from_hex("ffffffffffffffffffffffffffffffffffffffff"), addr: socket };
        bucket.add(test_node);
        fill(&mut bucket);
        bucket.add(test_node); //update
        println!("{:?}", bucket);
        assert_eq!(bucket.nodes.pop().unwrap().id, test_node.id);
    }

    fn fill(bucket: &mut Bucket) {
        for _ in 0..10_000 {
            bucket.add(NodeInfo {
                id:   U160::rand() >> (rand::random::<u8>() % 161),
                addr: SocketAddr::from_str("127.0.0.1:1337").unwrap(),
            })
        }
    }

    #[test]
    fn lookup() {
        let socket = SocketAddr::from_str("127.0.0.1:1337").unwrap();
        let mut bucket = Bucket::root(U160::empty(), 30);
        fill(&mut bucket);
        for _ in 0..60 {
            bucket.add(NodeInfo { id: U160::rand() >> (rand::random::<u8>() % 161), addr: socket })
        }

        let query = U160::rand() >> (rand::random::<u8>() % 161);
        let k_nearest = bucket.lookup(query);
        println!("Searching for: {:?}\nFound:\n{:?}", query, k_nearest);

        let query = U160::rand();
        let k_nearest = bucket.lookup(query);
        println!("Searching for: {:?}\nFound:\n{:?}", query, k_nearest);

        let k_nearest = bucket.lookup(U160::empty());
        println!("Searching for: {:?}\nFound:\n{:?}", U160::empty(), k_nearest);
    }

    #[test]
    fn serde() {
        let mut bucket = Bucket::root(U160::empty(), 8);
        fill(&mut bucket);
        let ser = bt_bencode::to_vec(&bucket).unwrap();
        println!("SER: {}", utils::safe_string_from_slice(&ser));
        let de: Bucket = bt_bencode::from_slice(&ser).unwrap();
        assert_eq!(de, bucket)
    }

    #[tokio::test]
    async fn file() {
        let mut bucket = Bucket::root(U160::empty(), 8);
        fill(&mut bucket);
        let path = "./bucket_test.ben";
        bucket.save_to_file(path.into()).await.unwrap();
        let b2 = Bucket::load_from_file(path.into()).await.unwrap();
        tokio::fs::remove_file(path).await.unwrap();
        assert_eq!(b2, bucket)
    }
}
