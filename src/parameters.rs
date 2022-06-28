use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "DHT", about = "do dht stuff")]
pub struct Parameters {
    #[structopt(short, long, default_value = "0.0.0.0:6881", about = "UDP ad.dr.es.ss:port")]
    pub bind_v4:   String,
    #[structopt(short = "6", long, about = "UDP [addr::esss]:port")]
    pub bind_v6:   Option<String>,
    #[structopt(long)]
    pub public_ip: Option<String>,

    #[structopt(short, long, parse(from_os_str), about = "node save state file")]
    pub state: Option<PathBuf>,

    #[structopt(
        short,
        long,
        default_value = "router.bittorrent.com:6881",
        about = "target of bootstrap or oneshot query (address:port)"
    )]
    pub peer: String,

    #[structopt(short, long, default_value = "500", about = "millis before a udp query times out")]
    pub timeout: u16,

    #[structopt(short, long = "no-verify-id", about = "allows nodes with ips that don't match their id")]
    pub no_verify_id: bool,

    #[structopt(long)]
    pub log_no_color:   bool,
    #[structopt(default_value = "Debug")]
    pub log_level:      log::LevelFilter,
    #[structopt(long)]
    pub log_std_level:  Option<log::LevelFilter>,
    #[structopt(long)]
    pub log_file_level: Option<log::LevelFilter>,
    #[structopt(long, default_value = "./logs/", about = "log dir")]
    pub log_dir:        String,
    #[structopt(long, default_value = "log-%Y-%m-%d-%H-%M-%S.txt", about = "log file with chrono timestamps")]
    pub log_file:       String,
}
