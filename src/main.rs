//! A IRC server

#![feature(collections)]
#![feature(test)]
#![feature(libc)]
#![feature(old_io)]
#![feature(alloc)]
#![feature(std_misc)]
#![feature(core)]
#![feature(box_syntax)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_features)]

#[macro_use]
extern crate log;
extern crate env_logger;


pub mod net;
pub mod protocol;
pub mod server;
pub mod message;
pub mod client;
pub mod user;
pub mod channel;

#[cfg(test)]
mod test;

fn main() {
    env_logger::init().unwrap();

    let _ = server::Server::new("localhost").map(|s| s.serve_forever()).unwrap();
}