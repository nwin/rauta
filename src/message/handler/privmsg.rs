use std::sync::Arc;
use std::ops::Range;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::PRIVMSG;
use client::{Client, MessageOrigin};
use client;
use user;
use server::Server;
use channel::{Channel, Member, Event};
use channel;
use misc::Receiver;
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for PRIVMSG command
///
/// `PRIVMSG <msgtarget> <text to be sent>`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    recv: misc::Receiver
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let recv = if let Some(receiver) = message.params().next() {
            match misc::verify_receiver(receiver) {
                Some(receiver) => receiver,
                None => return Err((
                    ERR_NOSUCHNICK,
                    ErrorMessage::WithSubject(format!("{}", String::from_utf8_lossy(receiver)), "No such nick/channel")
                ))
            }
        } else {
            return Err((
                ERR_NORECIPIENT, 
                ErrorMessage::Detailed(format!("No recipient given ({})", PRIVMSG))
            ))
        };
        Ok(Handler {
            msg: message,
            recv: recv
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        let msg = self.msg.params().nth(1);
        match self.recv {
            Receiver::Channel(ref name) => match server.channels().get(name) {
                Some(channel) => {
                    let msg = Arc::new(match msg {
                        Some(msg) => client.build_msg(PRIVMSG, &[name.as_bytes(), msg], MessageOrigin::User),
                        None => client.build_msg(PRIVMSG, &[name.as_bytes()], MessageOrigin::User),
                    });
                    channel.send(Event::Handle(box move |channel: &Channel| {
                        use channel::ChannelMode::*;
                        let maybe_member = channel.member_with_id(client.id());
                        if channel.has_flag(MemberOnly) || channel.has_flag(Moderated) {
                            match maybe_member {
                                Some(sender) => {
                                    if channel.has_flag(Moderated) && !sender.has_voice() {
                                        return // TODO error message if not NOTICE
                                    }
                                    for member in channel.members() {
                                        if member != sender {
                                            member.send(client::Event::SharedMessage(msg.clone()))
                                        }
                                    }
                                },
                                None => {
                                    return // TODO error message if not NOTICE
                                }
                            }
                        } else { // Message goes to everybody
                            match maybe_member {
                                Some(sender) => for member in channel.members() {
                                    if member != sender {
                                        member.send(client::Event::SharedMessage(msg.clone()))
                                    }
                                },
                                None => channel.broadcast_raw(msg)
                            }
                        }
                    }))
                },
                None => client.send_response(
                    ERR_NOSUCHNICK,
                    &[name, "No such nick/channel"]
                )
            },
            Receiver::Nick(ref nick) => match server.client_with_name(&nick) {
                Some(subject) => {
                    subject.send_raw(match msg {
                        Some(msg) => client.build_msg(PRIVMSG, &[nick.as_bytes(), msg], MessageOrigin::User),
                        None => client.build_msg(PRIVMSG, &[nick.as_bytes()], MessageOrigin::User),
                    })
                    
                },
                None => client.send_response(
                    ERR_NOSUCHNICK,
                    &[nick, "No such nick/channel"]
                )
            }
        }
    }
}