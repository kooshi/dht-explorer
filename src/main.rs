#![allow(dead_code)]
pub(crate) mod dht_node;
mod messenger;
mod options;
mod routing_table;
mod u160;
mod utils;
use messenger::{message::QueryMethod, Messenger};
use std::{net::SocketAddr, str::FromStr};
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

    let client_node = dht_node::DhtNode { id: U160::rand(), addr: SocketAddr::from_str(&opt.bind_address).unwrap() };
    let client = Messenger::new(client_node, opt.timeoutms, None).await?;

    let server_sock = SocketAddr::from_str(&opt.target_address).unwrap();

    let response = client.query(QueryMethod::Ping, server_sock).await;
    println!("RESPONSE: {:?}", response);

    Ok(())
}
