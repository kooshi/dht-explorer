#![allow(dead_code)]
#![feature(async_closure)]
#![feature(slice_as_chunks)]
#![feature(iter_next_chunk)]
#![feature(negative_impls)]
#![feature(array_windows)]
#![feature(map_first_last)]
mod messenger;
pub mod node;
mod node_info;
mod router;
pub mod u160;
pub mod utils;

const MAX_CONCURRENCY: u8 = 8;
const K_SIZE: u8 = 8;
