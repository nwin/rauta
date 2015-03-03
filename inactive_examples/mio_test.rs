#![feature(box_syntax)]
#![feature(net)]

extern crate mio;
extern crate rauta;

use std::default::Default;
use std::str::FromStr;

use mio::*;
use mio::net::tcp::{TcpSocket};

use rauta::client_io::{Event, Worker};
use std::thread::spawn;

fn main() {
	let addr = FromStr::from_str("127.0.0.1:6667").unwrap();

	// Setup the server socket
	let sock = TcpSocket::v4().unwrap();
	sock.bind(&addr).unwrap();
	let server = sock.listen(256).unwrap();

	// Create an event loop
	let mut event_loop = box EventLoop::<(), Event>::new().unwrap();
	let channel = event_loop.channel().clone();

	spawn(move || {
		let mut handler: Worker = Default::default();
		event_loop.run(&mut handler).unwrap();
	});

	for result in server.incoming() {
		match result {
			Ok(stream) => {
				println!("connected");
				let _ = channel.send(Event::NewConnection(stream));
			}
			Err(_) => () //Os(35) == would_block
		}
	}
}