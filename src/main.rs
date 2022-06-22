pub(crate) mod dht_node;
mod krpc;
mod options;
mod routing_table;
mod u160;
mod utils;
use krpc::message::{Message, MessageBase, Query, QueryMethod};
use log::{max_level, *};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
use tokio::time;
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

    let client_node = dht_node::DhtNode {
        id: U160::rand(),
        addr: SocketAddr::from_str(&opt.bind_address).unwrap(),
    };
    let client = krpc::KrpcService::new(client_node, opt.timeoutms, true).await?;

    let server_sock = SocketAddr::from_str(&opt.target_address).unwrap();

    let response = client.query(QueryMethod::Ping, server_sock).await?;
    println!("GOT IT {:?}", response);

    Ok(())
}
