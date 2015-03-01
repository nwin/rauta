use std::str;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{Command, ResponseCode, Message};
use protocol::ResponseCode::*;
use client::Client;
use server::Server;
use channel;

use super::{MessageHandler, ErrorMessage};

/// Handler for NICK command.
/// NICK nickname
#[derive(Debug)]
pub struct Handler {
    msg: Message,
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        Ok(Handler {
            msg: message
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        for (_, proxy) in server.channels().iter() {
            proxy.send(channel::Event::HandleMut(box move |channel| {

            }))
        }
    }
}

impl Handler {
    fn reason(&self) -> Option<&[u8]> {
        self.msg.params().next()
    }
}