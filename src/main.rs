#![allow(dead_code)]
mod messenger;
mod node;
pub(crate) mod node_info;
mod options;
mod routing_table;
mod u160;
mod utils;
use messenger::{message::QueryMethod, Messenger};
use simple_error::require_with;
use std::{net::{SocketAddr, ToSocketAddrs}, str::FromStr};
use structopt::StructOpt;
use u160::U160;

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

    let client_node = node_info::NodeInfo { id: U160::rand(), addr: SocketAddr::from_str(&opt.bind_v4)? };
    let client = Messenger::new(client_node, opt.timeout, None).await?;

    let response = client.query(QueryMethod::Ping, peer).await;
    println!("RESPONSE: {:?}", response);

    Ok(())
}
