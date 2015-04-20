//! IRC services
//! NickServ service
use std::str;
use std::fmt;
use std::any::Any;
use std::error::Error;
use std::ascii::AsciiExt;
use std::collections::HashMap;

use mio::Handler;

use client::Client;
use server::Server;
use client_io::Event;
use protocol::{Params, Message};
use protocol::Command::{PRIVMSG};

mod nickserv;
pub use self::nickserv::NickServ;

/// Service error
#[derive(Debug)]
pub enum ServiceError {
	DB // DB Error
}

/// Determines how to proceed after an service event handler has been processed
pub enum Action<'a> {
	Continue(&'a mut Server),
	Stop,
}

/// Service command argument
#[derive(Debug)]
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

/// Service command argument type
#[derive(Debug)]
pub enum ArgType {
	/// Text argument
	Text,
	/// Email address
	Email
}
pub use self::ArgType::*;

impl ArgType {
	fn check<'a>(&self, arg: &'a [u8]) -> Option<&'a str> {
		str::from_utf8(arg).ok()
	}
}

/// Determines the necessity of the argument
#[derive(Debug)]
pub enum Necessity {
	/// The argument is facultative
	Optional(ArgType),
	/// The argument is compulsory
	Obligatory(ArgType)
}
pub use self::Necessity::*;

type ActionFn = for <'a> fn(&mut Any, &'a mut Server, &Client, args: HashMap<String, String>) -> Action<'a>;

/// IRC service command
pub struct Command {
	name: String,
	args: Vec<Argument>,
	action: ActionFn,
}

impl fmt::Debug for Command {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    	write!(fmt, "Command {{ name: {}, args: {:?}, action: … }}", self.name, self.args)
    }
}

impl Command {
	fn new(name: &str, action: ActionFn) -> Command {
		Command {
			name: name.to_string(),
			args: Vec::new(),
			action: action
		}
	}

	/// Parses the command args and returns a HashMap containing at least the obligatory arguments
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

	/// Adds an argument to the command
	pub fn add_arg(mut self, name: &str, arg_type: Necessity) -> Command {
		self.args.push(Argument::new(name, arg_type));
		self
	}
}

/// Trait for IRC services
pub trait Service {
	fn add_command(&mut self, Command);
	fn commands(&self) -> &[Command];
	fn borrow_mut(&mut self) -> &mut Any;

	fn process_message<'a>(&mut self, message: &Message, server: &'a mut Server, client: &Client) -> Action<'a> {
		match message.command() {
			Some(PRIVMSG) => {
				let mut params = message.params();
				let handler = if let Some(cmd) = params.nth(1).and_then(|s| self.find_command(s)) {
					match cmd.parse_args(&mut params) {
						Ok(args) => Some((cmd.action, args)),
						Err(()) => None
					}
				} else {
					None
				};
				if let Some((action, args)) = handler {
					action(self.borrow_mut(), server, &client, args)
				} else {
					Action::Stop
				}
			}
			_ => Action::Stop,
		}
	}
	fn find_command(&self, cmd: &[u8]) -> Option<&Command> {
		if let Some(cmd) = str::from_utf8(cmd).ok().map(|v| v.to_ascii_uppercase()) {
			self.commands().iter().find(|c| c.name == cmd)
		} else {
			None
		}
	}
}