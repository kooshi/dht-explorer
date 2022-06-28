#![allow(dead_code)]
#![feature(async_closure)]
#![feature(slice_as_chunks)]
#![feature(iter_next_chunk)]
mod logging;
mod messenger;
mod node;
pub(crate) mod node_info;
mod parameters;
mod router;
mod u160;
mod utils;

use crate::logging::init_logging;
use fern::Dispatch;
use node::Node;
use parameters::Parameters;
use simple_error::require_with;
use std::{error::Error, net::{SocketAddr, ToSocketAddrs}, str::FromStr};
use structopt::StructOpt;
use tokio::sync::OnceCell;

static PARAMS: OnceCell<Parameters> = OnceCell::const_new();
#[macro_export]
macro_rules! param {
    () => {
        $crate::PARAMS.get().unwrap()
    };
}
#[macro_export]
macro_rules! init_fail {
    ($fallible:expr) => {
        match $fallible {
            Err(e) => {
                eprintln!("\x1b[31mERROR: Init failed with: {}\x1B[0m", e);
                panic!("init failed");
            }
            Ok(v) => v,
        }
    };
}
#[ctor::ctor]
fn init() {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Initialising...");
    #[cfg(not(test))]
    let p = init_fail!(Parameters::from_args_safe());
    #[cfg(test)]
    let p = init_fail!(Parameters::from_iter_safe(vec!["--log-level", "Off"]));
    init_fail!(PARAMS.set(p));
    init_logging();

    let peer = require_with!(param!().peer.to_socket_addrs()?.next(), "invalid peer address");
    let addr = SocketAddr::from_str(&param!().bind_v4)?;

    let node = Node::new(addr, false).await?;
    node.bootstrap(peer).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
    Ok(())
}
