use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "DHT", about = "do dht stuff")]
pub struct Opt {
    #[structopt(short, long, default_value = "10")]
    pub width: u16,

    #[structopt(short, long, default_value = "10")]
    pub height: u16,

    #[structopt(parse(from_os_str))]
    pub output: Option<PathBuf>,
}
