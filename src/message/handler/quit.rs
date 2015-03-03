use std::sync::Arc;

use protocol::{Command, ResponseCode, Message};
use client::{Client, MessageOrigin};
use server::Server;
use channel;

use super::{MessageHandler, ErrorMessage};

/// Handler for NICK command.
/// `NICK nickname`
#[derive(Debug, Clone)]
pub struct Handler {
    msg: Message,
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        Ok(Handler {
            msg: message
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        // Re-generate the message to ensure it is is well-formed
        let msg = Arc::new(match self.reason() {
            Some(reason) => client.build_msg(Command::QUIT, &[reason], MessageOrigin::User),
            None => client.build_msg(Command::QUIT, &[], MessageOrigin::User)
        });
        // TODO make this faster
        for (_, proxy) in server.channels().iter() {
            let msg = msg.clone();
            let id = client.id();
            proxy.with_ref_mut(move |channel| {
                if let Some(_) = channel.member_with_id(id) {
                    channel.broadcast_raw(msg);
                    channel.remove_member(&id);
                }
            })
        }
    }
}

impl Handler {
    fn reason(&self) -> Option<&[u8]> {
        self.msg.params().next()
    }
}