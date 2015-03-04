//! An IRC server

#![feature(collections)]
#![feature(test)]
#![feature(libc)]
#![feature(io)]
#![feature(old_io)]
#![feature(net)]
#![feature(std_misc)]
#![feature(core)]
#![feature(box_syntax)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_features)]
#![allow(missing_docs)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate rand;
extern crate mio;

pub mod net;
pub mod protocol;
pub mod server;
pub mod message_handler;
pub mod client;
pub mod user;
pub mod channel;
pub mod misc;
pub mod client_io;

#[cfg(test)]
mod test;

fn main() {
    env_logger::init().unwrap();

    let server = box server::Server::new("localhost");

    let _ = server.map(|mut s| s.run_mio()).unwrap();
}