//! A library for parsing irc messages

#![feature(collections)]
#![feature(test)]
#![feature(libc)]
#![feature(old_io)]
#![feature(alloc)]
#![feature(std_misc)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_features)]

#[macro_use]
extern crate log;
extern crate env_logger;

mod net;
mod protocol;
mod server;
mod message;
mod client;
mod user;
mod channel;

#[cfg(test)]
mod test;

fn main() {
    env_logger::init().unwrap();

    server::Server::new("localhost").map(|s| s.serve_forever()).unwrap();
}