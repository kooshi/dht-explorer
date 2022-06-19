use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "DHT", about = "do dht stuff")]
pub struct Opt {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    pub quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: usize,
    /// Timestamp (sec, ms, ns, none)
    #[structopt(short = "t", long = "timestamp", )]
    pub ts: Option<stderrlog::Timestamp>,
}
