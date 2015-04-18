use std::str;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use client::Client;
use server::Server;
use user;
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for NICK message
///
/// `NICK nickname
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
                        ErrorMessage::WithSubject(
                            String::from_utf8_lossy(nick).into_owned(),
                            "Erroneous nickname. Nickname has to be valid utf-8"
                        )
                    ))
                } else if misc::is_reserved_nick(nick) {
                    return Err((
                        ERR_ERRONEUSNICKNAME,
                        ErrorMessage::WithSubject(
                            String::from_utf8_lossy(nick).into_owned(),
                            "Erroneous nickname. Reserved nickname"
                        )
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
    fn invoke(self, server: &mut Server, client: Client) {
        use user::Status::*;
        let nick = self.nick();
        // Bypass borrow checker because of Rust issue #6393
        let server_ptr = server as *mut Server;
        // Note RFC issue #690, string has to be cloned twice now…
        // TODO: handle renames delete old entries and convert to lower case first
        match server.nicks_mut().entry(nick.to_string()) {
            // Unsafe reborrow because of Rust issue #6393
            Occupied(_) => unsafe {&*server_ptr}.send_response(
                &client, ERR_NICKNAMEINUSE,
                &[nick, "Nickname is already in use"]
            ),
            Vacant(entry) => {
                entry.insert(client.id());
                {let _ = client.info_mut().set_nick(nick.to_string());}
                let status = {
                    // Prevent dead-lock
                    client.info().status()
                };
                match status {
                    NameRegistered => {
                        {client.info_mut().set_status(Registered)}
                        // Unsafe reborrow because of Rust issue #6393
                        unsafe {&*server_ptr}.register(&client)
                    },
                    Negotiating(&NameRegistered) => {
                        client.info_mut().set_status(user::STATUS_NEG_REG)
                    },
                    Negotiating(_) => {
                        client.info_mut().set_status(user::STATUS_NEG_NICKREG)
                    }
                    _ => {
                        client.info_mut().set_status(NickRegistered)
                    }
                }
            }
        }
    }
}

impl Handler {
    fn nick(&self) -> &str {
    	str::from_utf8(self.msg.params().next().unwrap()).unwrap()
    }
}