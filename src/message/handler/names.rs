use std::str;
use std::ops::Range;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use client::Client;
use server::Server;
use user;
use channel::{Channel, Event};
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for NAMES command
///
/// `NAMES [ <channel> *( "," <channel> ) [ <target> ] ]`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    destinations: Vec<Range<usize>>
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let mut destinations = Vec::new();
        {
            let mut params = message.params();
            if let Some(channels) = params.next() {
                let mut start = 0;
                for channel_name in channels.split(|c| *c == b',') {
                    let len = channel_name.len();
                    if let Some(_) = misc::verify_channel(channel_name) {
                        destinations.push(start..start+len)
                    }
                    start += len + 1
                }
            }
        }
        Ok(Handler {
            msg: message,
            destinations: destinations
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
    	for chan_name in self.destinations() {
    		if let Some(channel) = server.channels().get(chan_name) {
    			let client = client.clone();
    			channel.send(Event::Handle(
    				box move |channel: &Channel| channel.send_names(&client)
    			))
    		}
    	}
    }
}

struct Destinations<'a> {
    h: &'a Handler,
    i: usize
}

impl<'a> Iterator for Destinations<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        use std::mem;
        let channels = self.h.msg.params().next().unwrap();
        if self.i < self.h.destinations.len() {
            let entry = &self.h.destinations[self.i];
            self.i += 1;
            Some(unsafe{mem::transmute(&channels[*entry])})
        } else {
        	None
        }
    }
}

impl Handler {
    fn destinations(&self) -> Destinations {
        Destinations {
            h: self,
            i: 0
        }
    }
}