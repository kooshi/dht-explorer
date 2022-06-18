
mod routing_table;
mod u160;
mod dht_node;

//mod disjoint_set;
mod options;
use structopt::StructOpt;
// use rand::prelude::SliceRandom;
// use disjoint_set::DisjointSet;
// use std::ops::Index;
// use rand::Rng;
// use std::path::PathBuf;


fn main() {

    //get options
    let opt = options::Opt::from_args();
    let (w, h) = (opt.width as usize, opt.height as usize);

    println!("Hello World");
}