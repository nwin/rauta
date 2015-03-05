use std::sync::Arc;
use std::ops::Range;
use std::iter::repeat;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::JOIN;
use client::{Client, MessageOrigin};
use server::Server;
use channel::{Channel, Member};
use misc;

use super::{MessageHandler, ErrorMessage, CommaSeparated, ParseError};

/// Handler for JOIN message
///
/// `JOIN <channel>{,<channel>} [<key>{,<key>}]`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    channels: CommaSeparated<str>,
    passwords: CommaSeparated<[u8]>,
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        // TODO filter out reserved names like "*"
        match CommaSeparated::verify(misc::verify_channel, message.params(), 0) {
            Ok(channels) => {
                let passwords = CommaSeparated
                    ::verify(|v| Some(v), message.params(), 1)
                    .unwrap_or(CommaSeparated::empty());
                Ok((channels, passwords))
            }
            Err(ParseError::Malformed(channel_name)) => Err((
                ERR_NEEDMOREPARAMS,
                ErrorMessage::WithSubject(
                    String::from_utf8_lossy(channel_name).into_owned(), 
                    "Invalid channel name"
                )
            )),
            Err(ParseError::TooMany) => Err((
                ERR_TOOMANYTARGETS, 
                ErrorMessage::WithSubject(
                    format!("{}", JOIN), 
                    "Number of targets is limited to 10"
                )
            )),
            Err(ParseError::Missing) => Err((
                ERR_NEEDMOREPARAMS, 
                ErrorMessage::WithSubject(format!("{}", JOIN), "No channel name given")
            )),
        }.map(|(channels, passwords)|
            Handler {
                msg: message,
                channels: channels,
                passwords: passwords,
            }
        )
    }
    fn invoke(self, server: &mut Server, client: Client) {
        use channel::ChannelMode::*;
        let tx = server.tx().clone();
        let msg = self.msg;
        let mut passwords = self.passwords.iter(msg.params());
        for channel in self.channels.iter(msg.params()) {
            let member = Member::new(client.clone());
            let password = passwords.next().map(|v| v.to_vec());
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
    if channel.is_invite_only() && !channel.is_invited(&member) {
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
    let msg = Arc::new(member.client().build_msg(JOIN, &[channel.name()], MessageOrigin::User));
    let id = member.id().clone();
    let _ = channel.remove_from_invite_list(member.id());
    let _ = channel.add_member(member);
    channel.broadcast_raw(msg);
    
    // Topic reply
    let member = channel.member_with_id(id).unwrap();
    if channel.topic() == "" {
        member.send_response(RPL_NOTOPIC, 
            &[channel.name(), "No topic set."]
        )
    } else {
        member.send_response(RPL_TOPIC, 
            &[channel.name(), channel.topic()]
        )
    } 
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
        let h = Handler::from_message(msg).ok().unwrap();
        let mut c = h.channels.iter(h.msg.params());
        let mut p = h.passwords.iter(h.msg.params());
        let name = c.next().unwrap();
        let pw = p.next();
        assert_eq!(name, "#hello");
        assert_eq!(pw, Some(b"pass"));
        let name = c.next().unwrap();
        let pw = p.next();
        assert_eq!(name, "#world");
        assert_eq!(pw, None);
    }
}
