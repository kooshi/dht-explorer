#![allow(dead_code)]
#![feature(async_closure)]
mod messenger;
mod node;
pub(crate) mod node_info;
mod options;
mod router;
mod u160;
mod utils;
use crate::{messenger::message::Message, node::Node};
use simple_error::require_with;
use std::{net::{SocketAddr, ToSocketAddrs}, str::FromStr};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = options::Opt::from_args();
    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .timestamp(opt.timestamps.unwrap_or(stderrlog::Timestamp::Off))
        .init()?;

    let peer = require_with!(opt.peer.to_socket_addrs()?.next(), "invalid peer address");
    let addr = SocketAddr::from_str(&opt.bind_v4)?;

    let node = Node::new(addr, true).await?;
    node.bootstrap(peer).await?;

    Ok(())
}
