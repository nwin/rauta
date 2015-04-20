
use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use client::Client;
use server::Server;
use channel::Channel;
use channel::ChannelMode::*;

use super::{MessageHandler, ErrorMessage};

/// Handles the WHO message
/// The reply consists of two parts:
/// 
/// ```
/// 352    RPL_WHOREPLY
///        "<channel> <user> <host> <server> <nick>
///        ( "H" / "G" > ["*"] [ ( "@" / "+" ) ]
///        :<hopcount> <real name>"
/// 
/// 315    RPL_ENDOFWHO
///        "<name> :End of WHO list"
/// ```
/// 
/// Unfortunately the RFC 2812 does not specify what H, G, *, @ or + mean.
/// @/+ is op/voice.
/// * is maybe irc op
/// H/G means here/gone in terms of the away status
/// WHO [<name> [<o>]]
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    op_only: bool,
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let op_only = match message.params().nth(1) {
            Some(val) => val == b"o",
            None => false
        };
        Ok(Handler {
            msg: message,
            op_only: op_only
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        match server.channels().get(&*String::from_utf8_lossy(self.mask())) {
            Some(channel) => {
                let op_only = self.op_only;
                let _ = channel.with_ref(move |channel| {
                    handle_who(channel, client, op_only)
                });
            },
            None => {} // handle later
        }
    }
}

impl Handler {
    fn mask(&self) -> &[u8] {
        self.msg.params().next().unwrap_or(b"0")
    }
}

pub fn handle_who(channel: &Channel, client: Client, op_only: bool) {
    let sender = channel.list_sender(&client, RPL_WHOREPLY, RPL_ENDOFWHO);
    if (channel.has_flag(Private) || channel.has_flag(Secret))
    && !channel.member_with_id(client.id()).is_some() {
        // Don't give information about this channel to the outside
        // this should also be ok for secret because RPL_ENDOFWHO is
        // always sent.
        drop(sender);
    } else {
        for member in channel.members() {
            if !op_only || member.is_op() {
                sender.feed_items(&[
                    member.username(),
                    member.hostname(),
                    member.client().server_name(),
                    member.nick(),
                    &*format!("{}{}{}", 
                        "H", // always here as long away is not implemented
                        "", // * is not supported yet
                        member.decoration()
                    ),
                    &*format!("0 {}", member.realname())
                ]);
            }
        }
    }
}
