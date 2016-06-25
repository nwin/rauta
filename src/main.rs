//! An IRC server

#![cfg_attr(test, feature(test))]

#![feature(box_syntax)]
#![feature(slice_patterns)]
#![feature(lookup_host)]
#![feature(fnbox)]
#![allow(unused_imports)]
#![allow(missing_docs)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate bytes;
extern crate num;
extern crate rand;
extern crate mio;

pub mod net;
pub mod services;
pub mod protocol;
pub mod server;
pub mod message_handler;
pub mod client;
pub mod user;
pub mod channel;
pub mod misc;
pub mod client_io;

#[cfg(test)]
pub mod test;

#[cfg(not(test))]
fn main() {
    env_logger::init().unwrap();

    let server = box server::Server::new("localhost");

    let _ = server.map(|mut s| s.run_mio()).unwrap();
}