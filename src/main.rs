//! An IRC server

#![cfg_attr(test, feature(test))]

#![feature(collections)]
#![feature(libc)]
#![feature(std_misc)]
#![feature(core)]
#![feature(box_syntax)]
#![feature(slice_patterns)]
#![feature(lookup_host)]
#![allow(unused_imports)]
#![allow(missing_docs)]

#[macro_use]
extern crate log;
extern crate env_logger;

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
mod test;

fn main() {
    env_logger::init().unwrap();

    let server = box server::Server::new("localhost");

    let _ = server.map(|mut s| s.run_mio()).ok().unwrap();
}