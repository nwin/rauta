//! NickServ service
use std::str;
use std::ascii::AsciiExt;
use std::collections::HashMap;

use client::Client;
use protocol::{Params, Message};
use protocol::Command::{PRIVMSG};

use super::*;

pub struct NickServ {
	commands: Vec<Command<NickServ>>
}

impl HasCommands for NickServ {
	fn add_command(&mut self, cmd: Command<Self>) {
		self.commands.push(cmd);
	}
	fn commands<'a>(&'a self) -> &[Command<Self>] {
		&*self.commands
	}
}

impl NickServ {
	pub fn new() -> NickServ {
		NickServ {
			commands: Vec::new()
		}
	}
	pub fn init(mut self) -> NickServ {
		self.add_command(
			Command::new("REGISTER", NickServ::register)
				.add_arg("password", Obligatory(Text))
				.add_arg("email", Obligatory(Email))
		);
		self
	}

	fn register(&mut self, _: &Client, _: HashMap<String, String>) {

	}
}