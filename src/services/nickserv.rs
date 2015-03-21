//! NickServ service
use std::str;
use std::any::Any;
use std::ascii::AsciiExt;
use std::collections::HashMap;

use client::Client;
use protocol::{Params, Message};
use protocol::Command::{PRIVMSG};

use super::*;

pub struct NickServ {
	commands: Vec<Command>
}

impl Service for NickServ {
	fn add_command(&mut self, cmd: Command) {
		self.commands.push(cmd);
	}
	fn commands<'a>(&'a self) -> &[Command] {
		&*self.commands
	}
	fn borrow_mut(&mut self) -> &mut Any {
		self
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

	fn register(this: &mut Any, _: &Client, _: HashMap<String, String>) {
		if let Some(mut this) = this.downcast_ref::<Self>() {

		}
	}
}