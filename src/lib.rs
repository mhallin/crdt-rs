#![feature(zero_one)]
#![feature(iter_arith)]

mod core;
mod counters;

pub use core::Operation;
pub use counters::{GCounter, PNCounter};
