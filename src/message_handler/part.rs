use std::ops::Range;
use std::sync::Arc;
use std::str;

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::PART;
use client::{Client, MessageOrigin};
use server::Server;
use misc;

use super::{MessageHandler, ErrorMessage, CommaSeparated, ParseError};

/// Handler for PART message
///
/// `PART <channel> *( "," <channel> ) [ <Part Message> ]`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    channels: CommaSeparated<str>,
    reason: Option<()>
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let channels = CommaSeparated::verify_no_error(misc::verify_channel, message.params(), 0);
        let reason = if let Some(_) = message.params().nth(1) {
            Some(())
        } else {
            None
        };
        if channels.iter(message.params()).count() == 0 {
            Err((ERR_NEEDMOREPARAMS, ErrorMessage::WithSubject(format!("{}", PART), "No channel name given")))
        } else {
            Ok(Handler {
                msg: message,
                channels: channels,
                reason: reason
            })   
        }
    }
    fn invoke(self, server: &mut Server, client: Client) {
        for chan_name in self.channels.iter(self.msg.params()) {
            if let Some(channel) = server.channels().get(chan_name) {
                let client = client.clone();
                let reason = self.reason().map(|v| v.to_vec());
                channel.with_ref_mut(move |channel| {
                    // Generate part msg
                    let msg = Arc::new(match reason {
                        Some(ref reason) => client.build_raw_msg(PART, &[channel.name().as_bytes(), &*reason], MessageOrigin::User),
                        None => client.build_msg(PART, &[channel.name()], MessageOrigin::User)
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

impl Handler {
    fn reason(&self) -> Option<&[u8]> {
        self.reason.map(|_| self.msg.params().nth(1).unwrap() )
    }
}