use std::io::prelude::*;
use std::thread::spawn;
use std::net::TcpStream;
use std::io::BufReader;
use server::Server;
use std::sync::{Once, ONCE_INIT};


pub use server::get_test_server;

/// A test client that communicates with the server
pub struct Client {
	stream: TcpStream,
	reader: BufReader<TcpStream>,
	nick: String
}

impl Client {
	pub fn registered(nick: &str) -> Client {
		let mut c = Client::new(nick.to_string());
		c.register();
		c
	}
	pub fn new(nick: String) -> Client {
		let stream = TcpStream::connect("127.0.0.1:6667").unwrap();
		let reader = BufReader::new(stream.try_clone().unwrap());
		Client {
			stream: stream,
			reader: reader,
			nick: nick
		}
	}
	pub fn register(&mut self) {
		let nick = self.nick.clone();
		self.send_msg("CAP END");
		self.send_msg(&*format!("NICK {}", nick));
		self.send_msg(&*format!("USER {} 0 * :Test user", nick));
		self.expect_begin(&*format!(":localhost 001 {}", nick));
	}
	pub fn send_msg(&mut self, msg: &str) {
		self.send_raw(msg.as_bytes());
		self.send_raw(b"\r\n");
	}
	pub fn send_raw(&mut self, msg: &[u8]) {
		self.stream.write(msg).unwrap();
	}
	pub fn read_raw(&mut self) -> Vec<u8> {
		let mut buf = Vec::new();
		self.reader.read_until(b'\n', &mut buf).unwrap();
		let len = buf.len();
		buf.truncate(len-2);
		buf
	}
	pub fn read_msg(&mut self) -> String {
		String::from_utf8(self.read_raw()).unwrap()
	}
	pub fn expect(&mut self, msg: &str) {
		assert_eq!(&*self.read_msg(), msg)
	}
	pub fn expect_begin(&mut self, msg: &str) {
		let response = self.read_msg();
		if !response.starts_with(msg)  {
			panic!("expected {} found {}", msg, response)
		}
	}
	pub fn skip_until(&mut self, msg: &str) {
		loop {
			let m = self.read_msg();
			if m.starts_with(msg) {
				break
			}
		}
	}
}

static SERVER: Once = ONCE_INIT;

pub fn run_server() {
	SERVER.call_once(|| {
		use std::thread::sleep_ms;
		spawn(move || {
			let mut server = get_test_server();
			server.run_mio().unwrap();
		});
		sleep_ms(1000);
	});
}
#[test]
/// simple test for test infrastructure
fn registration() {
	run_server();
	let mut client = Client::new("Nick".to_string());
	client.register();
}