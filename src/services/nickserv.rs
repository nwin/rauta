//! NickServ service
use std::str;
use std::any::Any;
use std::ascii::AsciiExt;
use std::collections::HashMap;

use super::sqlite3::{
    DatabaseConnection,
    SqliteResult,
};

use client::Client;
use server::Server;
use protocol::{Params, Message};
use protocol::Command::{PRIVMSG};

use super::{Command, Service, ServiceError, Action};
use super::{Obligatory, Text, Email};

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

	fn register<'a>(this: &mut Any, server: &'a mut Server, client: &Client, args: HashMap<String, String>) -> Action<'a> {
		if let Some(mut this) = this.downcast_ref::<Self>() {

		}
		Action::Continue(server)
	}
}