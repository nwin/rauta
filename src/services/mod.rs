//! IRC services
//! NickServ service
use std::str;
use std::ascii::AsciiExt;
use std::collections::HashMap;

use mio::Handler;

use client::Client;
use client_io::Event;
use protocol::{Params, Message};
use protocol::Command::{PRIVMSG};

//pub use self::nickserv::NickServ;

pub mod nickserv;

pub struct Argument {
	name: String,
	arg_type: Necessity,
}

impl Argument {
	fn new(name: &str, arg_type: Necessity) -> Argument {
		Argument {
			name: name.to_string(),
			arg_type: arg_type
		}
	}
}

pub enum ArgType {
	Text,
	Email
}
pub use self::ArgType::*;

impl ArgType {
	fn check<'a>(&self, arg: &'a [u8]) -> Option<&'a str> {
		str::from_utf8(arg).ok()
	}
}

pub enum Necessity {
	Optional(ArgType),
	Obligatory(ArgType)
}
pub use self::Necessity::*;

pub struct Command<Service> {
	name: String,
	args: Vec<Argument>,
	action: fn(&mut Service, &Client, args: HashMap<String, String>),
}

impl<Service> Command<Service> {
	fn new(name: &str, action: fn(&mut Service, &Client, HashMap<String, String>)) -> Command<Service> {
		Command {
			name: name.to_string(),
			args: Vec::new(),
			action: action
		}
	}

	pub fn parse_args(&self, params: &mut Params) -> Result<HashMap<String, String>, ()> {
		let mut args = HashMap::new();
		for arg in self.args.iter() {
			let param = params.next();
			match arg.arg_type {
				Obligatory(ref arg_type) => {
					match param {
						Some(p) => match arg_type.check(p) {
							Some(val) => {args.insert(arg.name.clone(), val.to_string());},
							None => return Err(())
						},
						None => return Err(())
					}
				},
				Optional(ref arg_type) => {
					param.and_then(
						|param| arg_type.check(param)).map(
							|v| args.insert(arg.name.clone(), v.to_string()
						)	
					);
				}
			}
		}
		Ok(args)
	}

	pub fn add_arg(mut self, name: &str, arg_type: Necessity) -> Command<Service> {
		self.args.push(Argument::new(name, arg_type));
		self
	}
}


pub trait Service: HasCommands {
	fn got_command(&self);
}

pub trait HasCommands: Sized {
	fn add_command(&mut self, Command<Self>);
	fn commands(&self) -> &[Command<Self>];

	fn process_message(&mut self, message: Message, client: &Client) {
		match message.command() {
			Some(PRIVMSG) => {
				let mut params = message.params();
				if let Some(cmd) = params.next().and_then(|s| self.find_command(s)) {
					match cmd.parse_args(&mut params) {
						Ok(args) => Some((cmd.action, args)),
						Err(()) => None
					}
				} else {
					None
				}.map(|(action, args)| action(self, &client, args));
			}
			_ => (),
		}
	}
	fn find_command(&self, cmd: &[u8]) -> Option<&Command<Self>> {
		if let Some(cmd) = str::from_utf8(cmd).ok().map(|v| v.to_ascii_uppercase()) {
			self.commands().iter().find(|c| c.name == cmd)
		} else {
			None
		}
	}
}