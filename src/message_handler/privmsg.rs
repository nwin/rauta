use std::sync::Arc;
use std::mem;

use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::{NOTICE, PRIVMSG};
use client::{Client, MessageOrigin};
use client_io;
use server::Server;
use misc::Receiver;
use misc;
use services::Action::Continue;

use super::{MessageHandler, ErrorMessage};

/// Handler for PRIVMSG and NOTICE messages
///
/// `PRIVMSG <msgtarget> <text to be sent>`
/// `NOTICE <msgtarget> <text>`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    recv: misc::Receiver
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let is_notice = message.command() == Some(NOTICE);
        let recv = if let Some(receiver) = message.params().next() {
            match misc::verify_receiver(receiver) {
                Some(receiver) => receiver,
                None => return Err((
                    ERR_NOSUCHNICK, if is_notice { ErrorMessage::None } else {
                    ErrorMessage::WithSubject(format!("{}", String::from_utf8_lossy(receiver)), "No such nick/channel")
                }))
            }
        } else {
            return Err((
                ERR_NORECIPIENT, if is_notice { ErrorMessage::None } else {
                ErrorMessage::Detailed(format!("No recipient given ({})", PRIVMSG))
            }))
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
                        Some(msg) => client.build_raw_msg(PRIVMSG, &[name.as_bytes(), msg], MessageOrigin::User),
                        None => client.build_msg(PRIVMSG, &[name], MessageOrigin::User),
                    });
                    channel.with_ref(move |channel| {
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
                                            member.send(client_io::Event::SharedMessage(member.id(), msg.clone()))
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
                                        member.send(client_io::Event::SharedMessage(member.id(), msg.clone()))
                                    }
                                },
                                None => channel.broadcast_raw(msg)
                            }
                        }
                    })
                },
                None => {
                    if !self.is_notice() { client.send_response(
                        ERR_NOSUCHNICK,
                        &[name, "No such nick/channel"]
                    )};
                    Ok(())
                }
            }.unwrap_or_else(|_| server.channel_lost(name)),
            Receiver::Nick(ref nick) => if let Continue(server) = server.with_service(
                nick,
                |service, server| service.process_message(&self.msg, server, &client)
            ) {
                match server.client_with_name(&nick) {
                    Some(subject) => {
                        subject.send_raw(match msg {
                            Some(msg) => client.build_raw_msg(
                                PRIVMSG, 
                                &[nick.as_bytes(), msg], 
                                MessageOrigin::User
                            ),
                            None => client.build_msg(
                                PRIVMSG, 
                                &[nick], 
                                MessageOrigin::User
                            ),
                        })
                    },
                    None => if ! self.is_notice() { client.send_response(
                        ERR_NOSUCHNICK,
                        &[nick, "No such nick/channel"]
                    )}
                }
            }
        }
    }
}

impl Handler {
    fn is_notice(&self) -> bool {
        self.msg.command() == Some(NOTICE)
    }
}

#[cfg(test)]
mod test {
    use test;
    #[test]
    fn privmsg_notice() {
        test::run_server();
        let mut client = test::Client::registered("privmsg_test");
        client.send_msg("NOTICE #nonexisting :Hello");
        client.send_msg("PRIVMSG #nonexisting2 :Hello");
        client.expect_begin(":localhost 401 privmsg_test #nonexisting2"); // no response for NOTICE
    }
}