use std::sync::Arc;
use std::ops::Range;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::JOIN;
use client::{Client, MessageOrigin};
use server::Server;
use channel::{Channel, Member};
use misc;

use super::{MessageHandler, ErrorMessage};

/// Handler for JOIN command
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    destinations: Vec<(Option<Range<usize>>, Option<Range<usize>>)>,
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
                    match misc::verify_channel(channel_name) {
                        Some(_) => {
                            destinations.push((Some(start..start+len), None));
                        },
                        None => return Err((
                            ERR_NEEDMOREPARAMS,
                            ErrorMessage::WithSubject(
                                String::from_utf8_lossy(channel_name).into_owned(), 
                                "Invalid channel name"
                            )
                        ))
                    }
                    start += len + 1
                }
                if let Some(passwords) = params.next() {
                    let mut start = 0;
                    for (channel, password) in destinations.iter_mut().zip(passwords.split(|c| *c == b',')) {
                        let len = password.len();
                        channel.1 = Some(start..start+len);
                        start += len + 1
                    }
                }
            } else {
                return Err((ERR_NEEDMOREPARAMS, ErrorMessage::WithSubject(format!("{}", JOIN), "No channel name given")))
            }
        }
        Ok(Handler {
            msg: message,
            destinations: destinations
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        use channel::ChannelMode::*;
        let tx = server.tx().clone();
        for (channel, password) in self.destinations() {
            let member = Member::new(client.clone());
            let password = password.map(|v| v.to_vec());
            match server.channels_mut().entry(channel.to_string()) {
                Occupied(entry) => entry.into_mut(),
                Vacant(entry) => {
                    let mut channel = Channel::new(channel.to_string());
                    channel.add_flag(TopicProtect);
                    channel.add_flag(MemberOnly);
                    entry.insert(channel.listen(tx.clone()))
                }
            }.with_ref_mut(move |channel| {
                handle_join(channel, member, password)
            })
        }
    }
}

struct Destinations<'a> {
    h: &'a Handler,
    i: usize
}

impl<'a> Iterator for Destinations<'a> {
    type Item = (&'a str, Option<&'a [u8]>);

    fn next(&mut self) -> Option<(&'a str, Option<&'a [u8]>)> {
        use std::mem;
        let mut p = self.h.msg.params();
        let names = p.next();
        let passwords = p.next();
        while self.i < self.h.destinations.len() {
            let entry = &self.h.destinations[self.i];
            self.i += 1;
            match entry {
                &(Some(ref r1), Some(ref r2)) => {
                    return Some((
                        unsafe{mem::transmute(&names.unwrap()[*r1])},
                        Some(&passwords.unwrap()[*r2])
                    ))
                }
                &(Some(ref r1), None) => {
                    return Some((
                        unsafe{mem::transmute(&names.unwrap()[*r1])},
                        None
                    ))
                }
                _ => ()
            }
        }
        None
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

fn handle_join(channel: &mut Channel, mut member: Member, password: Option<Vec<u8>>) {
    use channel::ChannelMode::*;
    match channel.password() {
        &Some(ref chan_pass) => if !match password { 
                Some(password) => &password == chan_pass,
                None => false } {
            member.send_response(ERR_BADCHANNELKEY,
                &[channel.name(),
                "Cannot join channel (+k)"]
            );
            return
        },
        &None => {},
    }
    if channel.member_with_id(member.id()).is_some() {
        // Member already in channel
        return
    }
    if member.mask_matches_any(channel.ban_masks()) 
       && !member.mask_matches_any(channel.except_masks()) {
        // Member banned
        channel.send_response(
            member.client(), 
            ERR_BANNEDFROMCHAN, 
            &["Cannot join channel (+b)"]
        );
        return
    }
    if channel.has_flag(InviteOnly) 
       && !member.mask_matches_any(channel.invite_masks()) {
        // Member not invited
        channel.send_response(
            member.client(), 
            ERR_INVITEONLYCHAN, 
            &["Cannot join channel (+i)"]
        );
        return
    }
    if channel.has_flag(UserLimit)
       && channel.limit().map_or(false, |limit| channel.member_count() + 1 >= limit) {
        // User limit reached
        channel.send_response(
            member.client(), 
            ERR_CHANNELISFULL, 
            &["Cannot join channel (+l)"]
        );
        return
    }
    // Give op to first user
    if channel.member_count() == 0 {
        member.promote(ChannelCreator);
        member.promote(OperatorPrivilege);
    }
    
    // Broadcast that a new member joined the channel and add him
    let msg = Arc::new(member.client().build_msg(JOIN, &[channel.name().as_bytes()], MessageOrigin::User));
    let id = member.id().clone();
    let _ = channel.add_member(member);
    channel.broadcast_raw(msg);
    
    // Topic reply
    let member = channel.member_with_id(id).unwrap();
    member.send_response(RPL_NOTOPIC, 
        &[channel.name(), "No topic set."]
    );
    channel.send_names(member.client())
}

#[cfg(test)]
mod tests {
    use super::super::MessageHandler;
    use super::Handler;
    use protocol::Message;
    /// Tests the mode parser
        
    #[test]
    fn parse_destinations() {
        let msg = Message::new(b"JOIN #hello,#world pass".to_vec()).unwrap();
        let handler = Handler::from_message(msg).ok().unwrap();
        let mut destinations = handler.destinations();
        let (name, pw) = destinations.next().unwrap();
        assert_eq!(name, "#hello");
        assert_eq!(pw, Some(b"pass"));
        let (name, pw) = destinations.next().unwrap();
        assert_eq!(name, "#world");
        assert_eq!(pw, None);
    }
}