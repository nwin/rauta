use std::sync::Arc;

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::TOPIC;
use client::{Client, MessageOrigin};
use server::Server;
use channel::Channel;
use channel::ChannelMode::TopicProtect;
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for TOPIC message
///
/// `TOPIC <channel> [ <topic> ]`
#[derive(Debug)]
pub struct Handler {
    msg: Message
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        // RUSTBUG return directly as soon as #6393 is fixed
        let ret = if let Some(channel_name) = message.params().next() {
            if let Some(_) = misc::verify_channel(channel_name) {
                Ok(())
            } else {
                Err((
                    ERR_NOSUCHCHANNEL,
                    ErrorMessage::WithSubject(
                        String::from_utf8_lossy(channel_name).into_owned(), 
                        "Invalid channel name"
                    )
                ))
            }
        } else {
            Err((
                ERR_NEEDMOREPARAMS,
                ErrorMessage::WithSubject(
                    format!("{}", TOPIC), 
                    "No channel name given"
                )
            ))
        };
        ret.map(|_| Handler {
            msg: message,
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        let topic = self.topic().map(|v| v.to_vec());
        match server.channels().get(self.name()) {
            Some(channel) => {
                channel.with_ref_mut(move |channel| {
                    let new_topic = match channel.member_with_id(client.id()) {
                        Some(member) => {
                            if channel.has_flag(TopicProtect) && !member.is_op() {
                                member.send_response(
                                    ERR_CHANOPRIVSNEEDED,
                                    &[channel.name(), "You are not a channel operator (channel is +t)."]
                                );
                                None
                            } else {
                                match topic {
                                    Some(topic) => {
                                        Some(topic)
                                    },
                                    None => {
                                        reply_topic(channel, member.client());
                                        None
                                    }
                                }
                            }
                        },
                        None => {
                            if channel.is_secret() && !channel.is_member(&client) {
                                client.send_response(
                                    ERR_NOSUCHCHANNEL,
                                    &[channel.name(), "No such channel"]
                                )
                            } else if topic.is_none() {
                                reply_topic(channel, &client)
                            } else {
                                client.send_response(
                                    ERR_NOTONCHANNEL,
                                    &[channel.name(), "You are not on this channel"]
                                )
                            }
                            None
                        }
                    };
                    if let Some(new_topic) = new_topic {
                        channel.broadcast_raw(Arc::new(client.build_msg(
                            TOPIC,
                            &[channel.name().as_bytes(), &*new_topic],
                            MessageOrigin::User
                        )));
                        channel.set_topic(new_topic);

                    }
                })
            }
            None => {
                client.send_response(ERR_NOSUCHCHANNEL, &[self.name(), "No such channel"])
            }
        }
    }
}

impl Handler {
    fn name(&self) -> &str {
        use std::mem::transmute;
        unsafe {
            transmute(self.msg.params().nth(0).unwrap())
        }

    }
    fn topic(&self) -> Option<&[u8]> {
        self.msg.params().nth(1)
    }
}

fn reply_topic(channel: &Channel, client: &Client) {
    match channel.topic() {
        /// TODO fix topic encoding!!
        topic if topic.len() > 0 => client.send_response(
            RPL_TOPIC, &[channel.name(), &*String::from_utf8_lossy(topic)]
        ),
        _ => client.send_response(
            RPL_NOTOPIC, &[channel.name(), "No topic it set"]
        ),
    }
}