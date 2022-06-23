use crate::{messenger::Messenger, node_info::NodeInfo, routing_table::bucket::Bucket};
use simple_error::SimpleResult;
use std::path::PathBuf;

pub struct Node {
    info:      NodeInfo,
    messenger: Messenger,
    routes:    Bucket,
}

impl Node {
    pub fn save_to_file(&self, path: PathBuf) -> SimpleResult<()> {
        todo!()
    }

    pub fn load_from_file(&self, path: PathBuf) -> SimpleResult<()> {
        todo!()
    }
}
