mod dht_node;
mod routing_table;
mod u160;
mod krpc;

//mod disjoint_set;
mod options;
use structopt::StructOpt;
use tokio;
use tokio::net::UdpSocket;
use log::{*, max_level};
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
    
    //let foo = UdpSocket::bind("0.0.0.0:1337").await?;
    
    println!("Hello World");

    Ok(())
}
