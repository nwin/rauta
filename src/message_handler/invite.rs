use std::ops::Range;
use std::sync::Arc;

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::INVITE;
use client::{Client, MessageOrigin};
use server::Server;
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for INVITE message
///
/// `INVITE <nickname> <channel>`
#[derive(Debug)]
pub struct Handler {
    msg: Message
}
// Possible return codes
// ERR_NEEDMOREPARAMS              ERR_NOSUCHNICK
// ERR_NOTONCHANNEL                ERR_USERONCHANNEL
// ERR_CHANOPRIVSNEEDED
// RPL_INVITING                    RPL_AWAY
impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
		if let Some(maybe_nick) = message.params().nth(0) {
			if misc::verify_nick(maybe_nick).is_none() {
				Err((ERR_NOSUCHNICK, 
                    ErrorMessage::WithSubject(
                        String::from_utf8_lossy(maybe_nick).into_owned(), 
                        "Invalid nick name"
                    )
                ))
			} else if let Some(maybe_chan) = message.params().nth(1) {
                if misc::verify_channel(maybe_chan).is_some() {
                    Ok(()) 
                } else {
                    Err((ERR_NOSUCHNICK, 
                        ErrorMessage::WithSubject(
                            String::from_utf8_lossy(maybe_nick).into_owned(), 
                            "Invalid channel name"
                        )
                    ))
                }
            } else {
                Err((ERR_NEEDMOREPARAMS, 
                    ErrorMessage::WithSubject(
                        format!("{}", INVITE), 
                        "Not enough parameters"
                    )
                ))

            }
		} else {
            Err((ERR_NEEDMOREPARAMS, 
                ErrorMessage::WithSubject(
                    format!("{}", INVITE), 
                    "Not enough parameters"
                )
            ))
        }.map(|_| Handler {
            msg: message
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        if let Some(target) = server.client_with_name(self.nick()) {
            if let Some(channel) = server.channels().get(self.channel()) {
                let target = target.clone();
                channel.with_ref_mut(move |channel| {
                    if if let Some(member) = channel.member_with_id(client.id()) {
                        if let Some(target_member) = channel.member_with_id(target.id()) {
                            client.send_response(
                                ERR_USERONCHANNEL,
                                &[target_member.nick(), channel.name(), "is already on channel"]
                            );
                            false
                        } else {
                            if channel.is_invite_only() && !member.is_op() {
                                client.send_response(
                                    ERR_USERONCHANNEL,
                                    &[channel.name(), "You're not channel operator"]
                                );
                                false
                            } else {
                                // All ok, send invitation
                                client.send_response(
                                    RPL_INVITING,
                                    &[channel.name(), &*target.nick()]
                                );
                                client.send_msg_from(
                                    INVITE, 
                                    &[&*target.nick(), channel.name()],
                                    &target
                                );
                                true // add to invite list (borrowing problem)
                            }
                        }
                    } else {
                        client.send_response(
                            ERR_NOTONCHANNEL,
                            &[channel.name(), "You're not on that channel"]
                        );
                        false
                    } { // Do this crazy if because of borrowing issues
                        channel.add_to_invite_list(target.id())
                    }
                })
            } else { // channel does not exist
                client.send_response(
                    RPL_INVITING,
                    &[self.channel(), &*target.nick()]
                );
                target.send_msg_from(
                    INVITE, 
                    &[&*target.nick(), self.channel()],
                    &client
                );
            }
        } else { // user does not exist
            client.send_response(
                ERR_NOSUCHNICK,
                &[self.nick(), "No such nick"]
            )
        }
    }
}

impl Handler {
    fn nick(&self) -> &str {
        use std::mem;
        unsafe { mem::transmute(self.msg.params().nth(0).unwrap()) }
    }
    fn channel(&self) -> &str {
        use std::mem;
        unsafe { mem::transmute(self.msg.params().nth(1).unwrap()) }
    }
}