pub(crate) mod dht_node;
mod krpc;
mod options;
mod routing_table;
mod u160;
mod utils;
use krpc::message::{Message, MessageData, Query, QueryMethod};
use log::{max_level, *};
use std::net::SocketAddrV4;
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

    let addr1 = "127.0.0.1:54321";
    let host_node = dht_node::DhtNode {
        id: U160::rand(),
        addr: SocketAddrV4::from_str(addr1).unwrap().into(),
    };
    let krpc = krpc::KrpcService::new(host_node, opt.timeoutms).await?;

    let addr2 = "127.0.0.1:12345";
    let host_node2 = dht_node::DhtNode {
        id: U160::rand(),
        addr: SocketAddrV4::from_str(addr2).unwrap().into(),
    };
    let krpc2 = krpc::KrpcService::new(host_node2, opt.timeoutms).await?;

    let mut count = 0;
    while count < 100 {
        let msg = Query::new(
            QueryMethod::Ping,
            MessageData::builder()
                .sender_id(U160::rand())
                .transaction_id(rand::random::<u32>().to_string())
                .destination_addr(SocketAddrV4::from_str(addr2).unwrap().into())
                .build(),
        );
        let response = krpc.query(msg).await?;
        println!("GOT IT {:?}", response);
        time::sleep(Duration::from_millis(10)).await;
        count += 1;
    }

    Ok(())
}
