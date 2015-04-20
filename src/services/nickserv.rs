//! NickServ service
use std::borrow::{Borrow, BorrowMut};
use std::str;
use std::any::Any;
use std::ascii::AsciiExt;
use std::collections::HashMap;

use client::{Client, MessageOrigin};
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
}

impl BorrowMut<Any> for NickServ {
	fn borrow_mut(&mut self) -> &mut Any {
		self
	}
}
impl Borrow<Any> for NickServ {
	fn borrow(&self) -> &Any {
		self
	}
}

impl NickServ {
	pub fn new() -> NickServ {
		NickServ {
			commands: Vec::new()
		}.init()
	}
	pub fn init(mut self) -> NickServ {
		self.add_command(
			Command::new("REGISTER", NickServ::register)
				.add_arg("password", Obligatory(Text))
				.add_arg("email", Obligatory(Email))
		);
		self
	}

	fn register<'a>(this: &mut Any, _: &'a mut Server, client: &Client, _: HashMap<String, String>) -> Action<'a> {
		if let Some(_) = this.downcast_ref::<Self>() {
			client.send_msg(PRIVMSG, &["cannot register new users at the moment"], MessageOrigin::Server)
		}
		Action::Stop
	}
}


#[cfg(test)]
mod test {
    use test;
    #[test]
    fn privmsg_notice() {
        test::run_server();
        let mut client = test::Client::registered("nickserv_test");
        client.send_msg("PRIVMSG NickServ REGISTER user email@email");
        client.expect_begin(":localhost PRIVMSG :cannot register new users at the moment");
    }
}