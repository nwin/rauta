use std::ops::Range;
use std::str;
    
use protocol::{ResponseCode, Message};
use client::Client;
use server::Server;
use misc;

use super::{MessageHandler, ErrorMessage, CommaSeparated, ParseError};

/// Handler for NAMES message
///
/// `NAMES [ <channel> *( "," <channel> ) [ <target> ] ]`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    destinations: CommaSeparated<str>
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let destinations = CommaSeparated::verify_no_error(misc::verify_channel, message.params(), 0);
        Ok(Handler {
            msg: message,
            destinations: destinations
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        let mut i = 0;
        for chan_name in self.destinations.iter(self.msg.params()) {
            if let Some(channel) = server.channels().get(chan_name) {
                let client = client.clone();
                let _ = channel.with_ref(move |channel| channel.send_names(&client));
            }
            i += 1;
        }
        if i == 0 {
            error!("NAMES with wildcard not implemented yet")
        }
    }
}