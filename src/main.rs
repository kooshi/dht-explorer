mod parameters;

use dht_explorer::node::Node;
use dht_explorer::u160::U160;
use dht_explorer::utils::LogErrExt;
use fern::Dispatch;
use log::info;
use parameters::Parameters;
use simple_error::{map_err_with, require_with, try_with, SimpleResult};
use std::error::Error;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use structopt::StructOpt;
use tokio::io::AsyncWriteExt;
use tokio::sync::OnceCell;
use tokio::time::Duration;
use tokio::{join, time};

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
                panic!("init");
            }
            Ok(v) => v,
        }
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let p = init_fail!(Parameters::from_args_safe());
    init_fail!(PARAMS.set(p));
    init_logging()?;

    let peer = require_with!(param!().peer.to_socket_addrs()?.next(), "invalid peer address");
    let addr = SocketAddr::from_str(&param!().bind_v4)?;
    let public_ip = try_with!(IpAddr::from_str(param!().public_ip.as_ref().unwrap()), "invalid public ip");
    let node = Node::new(addr, true, public_ip, "./target/state/".into()).await?;
    node.bootstrap(peer).await?;
    time::sleep(Duration::from_millis(10000)).await;
    // let found = node.find(U160::from_hex("B9FF4E7CE60DA918EB18D06AF1FDE0050D78E96E"), true).await;
    // info!("Found! {found:?}");
    // tokio::signal::ctrl_c().await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<U160>(100);
    let handle = tokio::spawn(async move {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("./target/state/infohashes.txt")
            .await
            .unwrap();
        while let Some(hash) = rx.recv().await {
            //info!("GOT ONE {hash}");
            file.write_all(format!("{}\n", hash.to_hex()).as_bytes()).await.log();
        }
    });
    node.infohash_sweep(tx).await;
    join!(handle).0.log();

    Ok(())
}

fn init_logging() -> SimpleResult<()> {
    let test_str = if cfg!(test) { "Test-" } else { "" };
    let fmt = Box::new(|color: bool| {
        // //Ironbow
        // let colors = |l: log::Level| match l {
        //     log::Level::Error => "230",
        //     log::Level::Warn => "221",
        //     log::Level::Info => "166",
        //     log::Level::Debug => "124",
        //     log::Level::Trace => "53",
        // };

        //Flame
        let colors = |l: log::Level| match l {
            log::Level::Error => "9",
            log::Level::Warn => "220",
            log::Level::Info => "228",
            log::Level::Debug => "230",
            log::Level::Trace => "248",
        };
        move |out: fern::FormatCallback, message: &std::fmt::Arguments, record: &log::Record| {
            let (ansi_pfx, ansi_sfx) = if color {
                (format!("\x1b[38;5;{}m", (colors)(record.level())), "\x1B[0m".to_owned())
            } else {
                ("".to_owned(), "".to_owned())
            };
            out.finish(format_args!(
                "{}{}[{}][{}][{}] {}{}",
                ansi_pfx,
                test_str,
                chrono::Local::now().format("%Y-%m-%d %T:%3f"),
                record.target(),
                record.level(),
                message,
                ansi_sfx
            ))
        }
    });

    let res = Dispatch::new()
        .chain(
            Dispatch::new()
                .format((fmt)(!param!().log_no_color))
                .level(param!().log_std_level.unwrap_or(param!().log_level))
                .level_for("dht_explorer::messenger", log::LevelFilter::Off)
                .level_for("dht_explorer::node", log::LevelFilter::Debug)
                .level_for("dht_explorer::router", log::LevelFilter::Off)
                .level_for("sled", log::LevelFilter::Off)
                .chain(std::io::stdout()),
        )
        .chain(
            Dispatch::new()
                .format((fmt)(false))
                .level(param!().log_file_level.unwrap_or(param!().log_level))
                .level_for("dht_explorer::router", log::LevelFilter::Off)
                .level_for("sled", log::LevelFilter::Off)
                .chain(init_fail!(fern::log_file(
                    chrono::Local::now()
                        .format(&(param!().log_dir.to_string() + test_str + &param!().log_file))
                        .to_string()
                ))),
        )
        .apply();
    map_err_with!(res, "failed to initialize logging")
}
