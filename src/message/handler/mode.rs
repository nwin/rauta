use std::sync::Arc;

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::MODE;
use client::{Client, MessageOrigin};
use user;
use server::Server;
use channel::Channel;
use channel;
use misc::Receiver;
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for MODE command
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
                None => if receiver.starts_with(b"#"){
                    return Err((
                        ERR_NOSUCHCHANNEL,
                        ErrorMessage::WithSubject(format!("{:?}", receiver), ":Invalid channel name")
                    ))
                } else {
                    return Err((
                        ERR_USERSDONTMATCH,
                        ErrorMessage::Plain(":Invalid user name")
                    ))
                }
            }
        } else {
            return Err((
                ERR_NEEDMOREPARAMS, 
                ErrorMessage::WithSubject(format!("{}", MODE), ":Not enough parameters")
            ))
        };
        Ok(Handler {
            msg: message,
            recv: recv
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        let msg = self.msg;
        match self.recv {
            Receiver::Channel(ref name) => match server.channels().get(name) {
                Some(channel) => channel.with_ref_mut(move |channel| {
                    handle_mode(channel, client, msg)
                }),
                None => client.send_response(
                    ERR_NOSUCHCHANNEL,
                    &[name.as_slice(), "No such channel"]
                )
            },
            Receiver::Nick(_) => error!("user modes not supported yet")
        }
    }
}

    
pub fn broadcast_change(channel: &channel::Channel, client: &Client, action: channel::Action,
                        flag: channel::ChannelMode, param: Option<&str>) {
    use channel::Action::*;
    let flag_str = match action {
        Add => "+",
        Remove => "-",
        Show => ""
    }.to_string() + &*(flag as u8 as char).to_string();
    let msg = Arc::new(match param {
        Some(param) => client.build_msg(
            MODE,
            &[channel.name().as_bytes(), flag_str.as_slice().as_bytes(), param.as_bytes()], 
            MessageOrigin::User
        ),
        None => client.build_msg(
            MODE,
            &[channel.name().as_bytes(), flag_str.as_slice().as_bytes()],
            MessageOrigin::User
        )
    });
    channel.broadcast_raw(msg);
}

/// Handles the channel mode message
pub fn handle_mode(channel: &mut channel::Channel, client: Client, message: Message) {
    use channel::ChannelMode::*;
    use channel::Action::*;
    // TODO broadcast changes
    // TODO send ERR_UNKNOWNMODE
    let is_op = { match channel.member_with_id(client.id()) {
        Some(member) => member.is_op(),
        None => false
    }};
    if message.params().count() > 1 {
        if !is_op { 
            client.send_response(ERR_CHANOPRIVSNEEDED,
                &[channel.name(), "You are not a channel operator"], 
            );
            return 
        }
        let mut params = message.params(); let _ = params.next();
        channel::modes_do(params, | action, mode, parameter | {
            match mode {
                AnonChannel | InviteOnly | Moderated | MemberOnly 
                | Quiet | Private | Secret | ReOpFlag | TopicProtect => {
                    match action {
                        Add => {
                            channel.add_flag(mode);
                            broadcast_change(channel, &client, action, mode, None)
                        },
                        Remove => {
                            channel.remove_flag(mode);
                            broadcast_change(channel, &client, action, mode, None)
                        },
                        Show => {} // ignore
                    }
                    
                },
                OperatorPrivilege | VoicePrivilege => {
                    if let Some(name) = parameter {
                        let nick = match channel.mut_member_with_nick(&String::from_utf8_lossy(name).to_string()) {
                            Some(member) => match action {
                                Add => {
                                    member.promote(mode);
                                    Some(member.nick().to_string())
                                },
                                Remove => {
                                    member.demote(mode);
                                    Some(member.nick().to_string())
                                },
                                Show => None // make not much sense
                            }, None => None
                        };
                        match nick {
                            Some(nick) => broadcast_change(
                                channel, &client, action, mode, Some(nick.as_slice())
                            ),
                            None => {}
                        }
                    }
                },
                ChannelKey => match action {
                    Add => if parameter.is_some() {
                        channel.set_password(parameter.and_then(|v| Some(v.to_vec())));
                        broadcast_change(channel, &client, action, mode, None)
                    },
                    Remove => {
                        channel.set_password(None);
                        broadcast_change(channel, &client, action, mode, None)
                    },
                    Show => {} // this might not be a good idea
                },
                UserLimit => match action {
                    Add => match parameter.and_then(|v| String::from_utf8_lossy(v).parse().ok()) {
                        Some(limit) => {
                            channel.set_limit(Some(limit));
                            broadcast_change(
                                channel, &client, action, mode, 
                                Some(limit.to_string().as_slice())
                            )
                        },
                        _ => {}
                    },
                    Remove => {
                        channel.set_limit(None);
                        broadcast_change(channel, &client, action, mode, None)
                    },
                    Show => {} // todo show
                },
                BanMask | ExceptionMask | InvitationMask => match parameter { 
                    Some(mask) => {
                        let host_mask = user::HostMask::new(
                            String::from_utf8_lossy(mask).to_string()
                        );
                        match mode {
                            BanMask => match action {
                                Add => {channel.add_ban_mask(host_mask);},
                                Remove => {channel.remove_ban_mask(host_mask);},
                                Show => {} // handled above
                            },
                            ExceptionMask => match action {
                                Add => {channel.add_except_mask(host_mask);},
                                Remove => {channel.remove_except_mask(host_mask);},
                                Show => {} // handled above
                            },
                            InvitationMask => match action {
                                Add => {channel.add_invite_mask(host_mask);},
                                Remove => {channel.remove_invite_mask(host_mask);},
                                Show => {} // handled above
                            },
                            _ => unreachable!()
                        }
                    },
                    None => {
                        let (start_code, end_code, masks) = match mode {
                            BanMask => (
                                RPL_BANLIST,
                                RPL_ENDOFBANLIST,
                                channel.ban_masks()
                            ),
                            ExceptionMask => (
                                RPL_EXCEPTLIST,
                                RPL_ENDOFEXCEPTLIST,
                                channel.except_masks()
                            ),
                            InvitationMask => (
                                RPL_INVITELIST,
                                RPL_ENDOFINVITELIST,
                                channel.invite_masks()
                            ),
                            _ => unreachable!()
                        };
                        let sender = channel.list_sender(
                            &client, start_code, end_code
                        );
                        for mask in masks.iter() {
                            sender.feed_line_single(mask.as_str())
                        }
                        sender.end_of_list()
                    }
                    
                },
                ChannelCreator => {
                    match action {
                        Add | Remove => {} // This is can't be set after channel creation 
                        Show => {} // TODO show
                    }
                },
            }
        });
    } else {
        // TODO secret channel??
        // TODO things with parameters?
        client.send_response(RPL_CHANNELMODEIS,
            &[channel.name(), ("+".to_string() + &*channel.flags()).as_slice()]
        )
    }
}