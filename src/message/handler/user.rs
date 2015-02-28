use std::ops::Range;
use std::str;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::USER;
use client::Client;
use server::Server;
use super::{MessageHandler, ErrorMessage};
use user::Status;

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
                    String::from_utf8_lossy(params.next().unwrap()).into_owned(),
                    String::from_utf8_lossy(params.nth(2).unwrap()).into_owned()
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
    fn invoke(self, server: &mut Server, client: Client) {
        let reg_new = {
            let ref mut info = client.info_mut();
            if info.status() != Status::Registered {
                info.set_user(self.username);
                info.set_realname(self.realname);
                let status = if info.nick() == "*" {
                    Status::RegistrationPending
                } else {
                    Status::Registered
                };
                info.set_status(status);
                true
            } else {
                false
            }
        };
        let nick_ok = {
            client.info().nick() != "*"
        };
        if nick_ok && reg_new {
            server.register(&client)
        } else if !reg_new {
            server.send_response(
                &client, 
                ERR_ALREADYREGISTRED, 
                &["Unauthorized command (already registered)"]
            )
        }
        
    }
}