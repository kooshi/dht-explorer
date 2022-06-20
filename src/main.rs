mod dht_node;
mod krpc;
mod routing_table;
mod u160;

//mod disjoint_set;
mod options;
use core::slice;
use std::{net::SocketAddrV4};
use std::str::FromStr;

use log::{max_level, *};
use structopt::StructOpt;
use tokio::net::UdpSocket;
use krpc::message::{Message,MessageKind,QueryMethod,ResponseKind};
use u160::U160;

//use crate::krpc::kmsg::socket_addr_wrapper::SocketAddrWrapper;
// use rand::prelude::SliceRandom;
// use disjoint_set::DisjointSet;
// use std::ops::Index;
// use rand::Rng;
// use std::path::PathBuf;

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

    let socket = UdpSocket::bind(opt.bind_address).await?;
    socket.connect(&opt.target_address).await?;

    // let mut msg:krpc::kmsg::KMessage = Default::default();
    // msg.arguments = Some(Default::default());
    // msg.read_only = Some(true);
    // msg.query_method = Some("find_node".to_string());
    // msg.message_type = "q".to_string();
    // msg.transaction_id = "testing".to_string();
    // msg.peer_ip = Some(SocketAddrWrapper{socket_addr: Some(SocketAddrV4::from_str(&opt.public_address).unwrap().into())});
    // msg.arguments.as_mut().unwrap().id = u160::U160::rand();
    // msg.arguments.as_mut().unwrap().target = Some(u160::U160::rand());

    let msg = Message::builder()
    .read_only()
    .sender_id(U160::rand())
    .transaction_id("testing".to_string())
    .destination_addr(SocketAddrV4::from_str(&opt.target_address).unwrap().into())
    //.kind(MessageKind::Query(QueryMethod::Ping))
    .kind(MessageKind::Query(QueryMethod::FindNode(U160::rand())))
    //.kind(MessageKind::Query(QueryMethod::GetPeers(U160::rand())))
    .build();

    println!("{:#?}",msg);
    let msg = bt_bencode::to_vec(&msg).unwrap();

    println!("SENDING: {}", krpc::message::kmsg::safe_string_from_slice(&msg));
    socket.send(&msg).await?;

    let mut buf = [0; 1024];
    let len = socket.recv(&mut buf).await?;
    println!("**************************************************");
    println!("RECIEVED: {}", krpc::message::kmsg::safe_string_from_slice(&buf[..len]));
    println!("Base64: {}", base64::encode(&buf[..len]));
    println!();
    let result = bt_bencode::from_slice::<krpc::message::kmsg::KMessage>(&buf[..len]).unwrap();
    println!("{:#?}",result);
    let result = Message::from_kmsg(result);
    println!("{:#?}",result);
    Ok(())
}
