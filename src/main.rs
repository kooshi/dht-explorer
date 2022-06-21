mod dht_node;
mod krpc;
mod options;
mod routing_table;
mod u160;
mod utils;
use krpc::message::{Message, MessageKind, QueryMethod};
use log::{max_level, *};
use std::net::SocketAddrV4;
use std::str::FromStr;
use structopt::StructOpt;
use u160::U160;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = options::Opt::from_args();
    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .timestamp(opt.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()?;

    match max_level() {
        LevelFilter::Error => error!("error logs enabled"),
        LevelFilter::Warn => warn!("warning logs enabled"),
        LevelFilter::Info => info!("info logs enabled"),
        LevelFilter::Debug => debug!("debug logs enabled"),
        LevelFilter::Trace => trace!("trace logs enabled"),
        LevelFilter::Off => (),
    };

    let host_node = dht_node::DhtNode {
        id: U160::rand(),
        addr: SocketAddrV4::from_str(&opt.bind_address).unwrap().into(),
    };
    let krpc = krpc::KrpcService::new(host_node).await.unwrap();

    let msg = Message::builder()
        .read_only()
        .sender_id(U160::rand())
        .transaction_id("testing".to_string())
        .destination_addr(SocketAddrV4::from_str(&opt.target_address).unwrap().into())
        .kind(MessageKind::Query(QueryMethod::Ping))
        //.kind(MessageKind::Query(QueryMethod::FindNode(U160::rand())))
        //.kind(MessageKind::Query(QueryMethod::GetPeers(U160::rand())))
        .build();

    println!("{:?}", msg);
    krpc.send_with_continue(msg, Box::new(|m| println!("Success! {}", m.kind())))
        .await;

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");
    Ok(())
}
