use std::fmt::Result;

use rand::seq::index;

use crate::{dht_node::DhtNode, u160::U160};

const MAX_BUCKET_INDEX: u8 = 159;

#[derive(Debug)]
pub struct Bucket<'a> {
    host_node: &'a DhtNode,
    bucket_index: u8,
    k_size: u8,
    nodes: Vec<DhtNode>,
    next_bucket: Option<Box<Bucket<'a>>>,
}

impl<'a> Bucket<'a> {
    pub fn root(host_node: &'a DhtNode, k_size: u8) -> Self {
        Self {
            host_node,
            k_size,
            nodes: Vec::with_capacity(k_size as usize),
            next_bucket: None,
            bucket_index: 0,
        }
    }

    pub fn add(&mut self, node: DhtNode) {
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

    fn add_or_update(&mut self, node: DhtNode) {
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

    pub fn lookup(&self, id: U160) -> Vec<DhtNode> {
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
            k_nearest.extend(
                self.next_bucket
                    .as_ref()
                    .unwrap()
                    .lookup(id)
                    .iter()
                    .take(gap)
                    .cloned()
                    .collect::<Vec<_>>(),
            );
        }

        k_nearest
    }

    fn make_next_bucket(&mut self) -> bool {
        assert!(self.next_bucket.is_none());
        if self.bucket_index == MAX_BUCKET_INDEX {
            return false;
        }

        self.next_bucket = Some(Box::new(Bucket {
            host_node: self.host_node,
            k_size: self.k_size,
            nodes: Vec::with_capacity(self.k_size as usize),
            next_bucket: None,
            bucket_index: self.bucket_index + 1,
        }));

        let mut index = 0;
        while index < self.nodes.len() {
            if self.belongs_here(&self.nodes[index]) {
                index += 1;
            } else {
                self.next_bucket
                    .as_mut()
                    .unwrap()
                    .add(self.nodes.swap_remove(index));
            }
        }
        true
    }

    fn belongs_here(&self, node: &DhtNode) -> bool {
        self.host_node.distance(node).get_bit(self.bucket_index)
    }
    fn id_belongs_here(&self, id: U160) -> bool {
        self.host_node.id.distance(id).get_bit(self.bucket_index)
    }
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddrV4, str::FromStr};

    use crate::{dht_node::DhtNode, u160::U160};

    use super::Bucket;

    #[test]
    fn fill() {
        let socket = std::net::SocketAddr::from(SocketAddrV4::from_str("127.0.0.1:1337").unwrap());
        let host = DhtNode {
            id: U160::empty(),
            addr: socket,
        };
        let mut bucket = Bucket::root(&host, 8);
        let test_node = DhtNode {
            id: U160::from_hex("ffffffffffffffffffffffffffffffffffffffff"),
            addr: socket,
        };
        bucket.add(test_node);
        for _ in 0..1_000_000 {
            bucket.add(DhtNode {
                id: U160::rand() >> (rand::random::<u8>() % 161),
                addr: socket,
            })
        }
        bucket.add(test_node); //update
        println!("{:?}", bucket);
        assert_eq!(bucket.nodes.pop().unwrap().id, test_node.id);
    }

    #[test]
    fn lookup() {
        let socket = std::net::SocketAddr::from(SocketAddrV4::from_str("127.0.0.1:1337").unwrap());
        let host = DhtNode {
            id: U160::empty(),
            addr: socket,
        };
        let mut bucket = Bucket::root(&host, 30);
        for _ in 0..60 {
            bucket.add(DhtNode {
                id: U160::rand() >> (rand::random::<u8>() % 161),
                addr: socket,
            })
        }

        let query = U160::rand() >> (rand::random::<u8>() % 161);
        let k_nearest = bucket.lookup(query);
        println!("Searching for: {:?}\nFound:\n{:?}", query, k_nearest);

        let query = U160::rand();
        let k_nearest = bucket.lookup(query);
        println!("Searching for: {:?}\nFound:\n{:?}", query, k_nearest);

        let k_nearest = bucket.lookup(U160::empty());
        println!(
            "Searching for: {:?}\nFound:\n{:?}",
            U160::empty(),
            k_nearest
        );
    }
}
