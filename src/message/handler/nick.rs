use std::str;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use client::Client;
use server::Server;
use super::{MessageHandler, ErrorMessage};

/// Handler for NICK command.
/// NICK nickname
#[derive(Debug)]
pub struct Handler {
    msg: Message
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
    	if let Some(_) = message.params().next() {
            // _ + repetition because of Rust issue #6393
            if let Some(nick) = message.params().next() {
                if let Err(_) = str::from_utf8(nick) {
                    return Err((
                        ERR_ERRONEUSNICKNAME,
                        ErrorMessage::Plain("Erroneous nickname. Nickname has to be valid utf-8")
                    ))
                }
            }
    		Ok(Handler {
    			msg: message
    		})
    	} else {
    		Err((ERR_NONICKNAMEGIVEN, ErrorMessage::Plain("No nickname given")))
    	}
    }
    fn invoke(&self, server: &mut Server, client: Client) {
        let nick = self.nick();
        // Note RFC issue #690, string has to be cloned twice now…
        match server.nicks_mut().entry(nick.to_string()) {
            // Unsafe reborrow because of Rust issue #6393
            Occupied(_) => unsafe {(&*(server as *mut Server))}.send_response(
                &client, ERR_NICKNAMEINUSE,
                &[nick, "Nickname is already in use"]
            ),
            Vacant(entry) => {
                entry.insert(client.id());
                client.info_mut().name = nick.to_string();
            }
        }
    }
}

impl Handler {
    fn nick(&self) -> &str {
    	use std::mem::transmute;
    	unsafe { transmute(self.msg.params().next().unwrap()) }
    }
}