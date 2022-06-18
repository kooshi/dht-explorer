
use rand::seq::index;

use crate::dht_node::DhtNode;

const MAX_BUCKET_INDEX:u8 = 159;

pub struct Bucket<'a> {
    host_node:&'a DhtNode,
    bucket_index: u8,
    max_nodes_per_bucket:u8,
    nodes:Vec<DhtNode>,
    next_bucket:Option<Box<Bucket<'a>>>
}

impl<'a> Bucket<'a> {
    pub fn root(host_node:&'a DhtNode, max_nodes_per_bucket:u8) -> Self {
        Self { host_node, max_nodes_per_bucket, nodes: Vec::with_capacity(max_nodes_per_bucket as usize), next_bucket: None, bucket_index: 0 }
    }

    pub fn add(&mut self, node:DhtNode) {
        if self.next_bucket.is_none() && self.nodes.len() < self.max_nodes_per_bucket.into() {
            self.nodes.push(node);
            return;
        }
        if self.next_bucket.is_none() && self.nodes.len() == self.max_nodes_per_bucket.into() {
            if !self.make_next_bucket() {
                return;
            }
        }
        if !self.belongs_here(&node) {
            self.next_bucket.as_mut().unwrap().add(node);
            return;
        }
        if self.nodes.len() < self.max_nodes_per_bucket.into() {
            self.nodes.push(node);
        }
    }

    fn make_next_bucket(&mut self) -> bool {
        assert!(self.next_bucket.is_none());
        if self.bucket_index == MAX_BUCKET_INDEX {
            return false;
        }

        self.next_bucket = Some(Box::new(Bucket{ 
            host_node: self.host_node, 
            max_nodes_per_bucket: self.max_nodes_per_bucket, 
            nodes: Vec::with_capacity(self.max_nodes_per_bucket as usize), 
            next_bucket: None, 
            bucket_index: self.bucket_index+1 
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

    pub fn belongs_here(&self, node:&DhtNode)->bool{
        self.host_node.distance(node).get_bit(self.bucket_index)
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
        let host = DhtNode { id: U160::empty(), addr: socket };
        let mut bucket = Bucket::root(&host, 3);
        for _ in 0..1_000_000 {
            bucket.add(DhtNode { id: U160::new(), addr: socket })
        }
        for _ in 0..100_000 {
            bucket.add(DhtNode { id: U160::empty(), addr: socket })
        }
        for _ in 0..1_000_000 {
            bucket.add(DhtNode { id: U160::new(), addr: socket })
        }
    }

    // #[test]
    // fn union() {
    //     let mut d = DisjointSet::with_size(10);
    //     for i in 1..6 {
    //         assert!(d.try_union(i, i - 1));
    //     }
    //     for i in 7..10 {
    //         assert!(d.try_union(i, i - 1));
    //     }
    //     assert_eq!(d.size(0), 6);
    //     assert_eq!(d.size(9), 4);
    //     assert!(d.try_union(3, 8));
    //     for i in 0..d.len() {
    //         assert_eq!(d.size(i), 10);
    //     }
    // }
}