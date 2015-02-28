use std::ops::Range;
use std::str;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::USER;
use client::Client;
use server::Server;
use super::{MessageHandler, ErrorMessage};

/// Handler for NICK command.
/// NICK nickname
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    username: String,
    realname: String
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        if message.params().count() == 4 {
            let (username, realname) = {
                let mut params = message.params();
                (
                    String::from_utf8_lossy(params.next().unwrap()).to_string(),
                    String::from_utf8_lossy(params.nth(2).unwrap()).to_string()
                )
            };
            Ok(Handler {
                msg: message,
                username: username,
                realname: realname
            })
        } else {
            Err((
                ERR_NEEDMOREPARAMS,
                ErrorMessage::WithSubject(format!("{}", USER), "Not enough parameters")
            ))
        }
    }
    fn invoke(&self, server: &mut Server, client: Client) {
    }
}