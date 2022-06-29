#![allow(dead_code)]
#![feature(async_closure)]
#![feature(slice_as_chunks)]
#![feature(iter_next_chunk)]
mod messenger;
pub mod node;
mod node_info;
mod router;
mod u160;
mod utils;

const MAX_CONCURRENCY: u8 = 8;
const K_SIZE: u8 = 8;