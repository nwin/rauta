use protocol::{ResponseCode, Message};
use protocol::Command::USER;
use client::Client;
use server::Server;
use super::{MessageHandler, ErrorMessage};
use user;

/// Handler for NICK command.
/// NICK nickname
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    username: String,
    realname: String
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        use protocol::ResponseCode::*;
        if message.params().count() == 4 {
            let (username, realname) = {
                let mut params = message.params();
                (
                    String::from_utf8_lossy(params.next().unwrap()).into_owned(),
                    String::from_utf8_lossy(params.nth(2).unwrap()).into_owned()
                )
            };
            Ok(Handler {
                msg: message,
                username: username,
                realname: realname
            })
        } else {
            Err((
                ERR_NEEDMOREPARAMS,
                ErrorMessage::WithSubject(format!("{}", USER), "Not enough parameters")
            ))
        }
    }
    fn invoke(self, server: &mut Server, client: Client) {
        use protocol::ResponseCode::*;
        use user::Status::*;
        let status = {
            // Prevent dead-lock
            client.info().status()
        };
        match status {
            Registered | NameRegistered |
            Negotiating(&NameRegistered) | Negotiating(&Registered) => {
                server.send_response(
                    &client, 
                    ERR_ALREADYREGISTRED, 
                    &["Unauthorized command (already registered)"]
                )
            },
            status => {
                {
                    let ref mut info = client.info_mut();
                    info.set_user(self.username);
                    info.set_realname(self.realname);
                }
                match status {
                    Negotiating(&NickRegistered) => {
                        client.info_mut().set_status(user::STATUS_NEG_REG)
                    }
                    Negotiating(_) => {
                        client.info_mut().set_status(user::STATUS_NEG_NAMEREG)
                    }
                    NickRegistered => {
                        {client.info_mut().set_status(Registered)}
                        server.register(&client)
                    }
                    _ => {
                        client.info_mut().set_status(NameRegistered)
                    }
                }
            },
        }
    }
}