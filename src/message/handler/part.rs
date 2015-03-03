use std::ops::Range;
use std::sync::Arc;

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::PART;
use client::{Client, MessageOrigin};
use server::Server;
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for PART command
///
/// `PART <channel> *( "," <channel> ) [ <Part Message> ]`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    destinations: Vec<Range<usize>>,
    reason: Option<()>
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let mut destinations = Vec::new();
        let reason = {
            let mut params = message.params();
            if let Some(channels) = params.next() {
                let mut start = 0;
                for channel_name in channels.split(|c| *c == b',') {
                    let len = channel_name.len();
                    if let Some(_) = misc::verify_channel(channel_name) {
                        destinations.push(start..start+len)
                    } else {
                        return Err((
                            ERR_NEEDMOREPARAMS,
                            ErrorMessage::WithSubject(
                                String::from_utf8_lossy(channel_name).into_owned(), 
                                "Invalid channel name"
                            )
                        ))
                    }
                    start += len + 1
                }
            }
            if let Some(_) = params.next() {
                Some(())
            } else {
                None
            }
        };
        Ok(Handler {
            msg: message,
            destinations: destinations,
            reason: reason
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        for chan_name in self.destinations() {
            if let Some(channel) = server.channels().get(chan_name) {
                let client = client.clone();
                let reason = self.reason().map(|v| v.to_vec());
                channel.with_ref_mut(move |channel| {
                    // Generate part msg
                    let msg = Arc::new(match reason {
                        Some(ref reason) => client.build_msg(PART, &[channel.name().as_bytes(), &*reason], MessageOrigin::User),
                        None => client.build_msg(PART, &[channel.name().as_bytes()], MessageOrigin::User)
                    });
                    let id = client.id();
                    if let Some(_) = channel.member_with_id(id) {
                        channel.broadcast_raw(msg);
                        channel.remove_member(&id);
                    } else {
                        client.send_response(
                            ERR_NOTONCHANNEL, 
                            &[channel.name(), "You're not on that channel"]
                        )
                    }
                })
            } else {
                client.send_response(
                    ERR_NOSUCHCHANNEL, 
                    &[chan_name, "No such channel"]
                );
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
    fn reason(&self) -> Option<&[u8]> {
        self.reason.map(|_| self.msg.params().nth(1).unwrap() )
    }
}