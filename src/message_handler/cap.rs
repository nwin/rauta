use std::ascii::AsciiExt;
use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::CAP;
use client::Client;
use server::Server;
use super::{MessageHandler, ErrorMessage};
use user;

/// Handler for CAP message
///
/// `CAP subcommand [params]`
#[derive(Debug)]
pub struct Handler {
    msg: Message,
    args: Option<usize>
}

/// CAP subcommands
enum Subcommand {
    LS,
    LIST,
    REQ,
    ACK,
    NAK,
    CLEAR,
    END,
}
use self::Subcommand::*;

impl Subcommand {
    fn from_slice(slice: &[u8]) -> Option<Subcommand> {
        Some(match slice {
            b"LS"    => LS,
            b"LIST"  => LIST,
            b"REQ"   => REQ,
            b"ACK"   => ACK,
            b"NAK"   => NAK,
            b"CLEAR" => CLEAR,
            b"END"   => END,
            _ => return None
        })
    }
    fn as_slice(&self) -> &'static str {
        match *self {
            LS => "LS",
            LIST => "LIST",
            REQ => "REQ",
            ACK => "ACK",
            NAK => "NAK",
            CLEAR => "CLEAR",
            END => "END"
        }
    }
    fn as_bytes(&self) -> &'static [u8] {
        self.as_slice().as_bytes()
    }
}

impl MessageHandler for Handler {
    fn from_message(message: Message) -> Result<Handler, (ResponseCode, ErrorMessage)> {
        let args = {
            let mut params = message.params();
            if let Some(ref param) = params.next() {
                if Subcommand::from_slice(param).is_none() {
                    return Err((ERR_INVALIDCAPCMD, ErrorMessage::WithSubject(
                        format!("{:?}", param), "Invalid subcommand."
                    )))
                }
            } else {
                return Err((ERR_INVALIDCAPCMD, ErrorMessage::Plain(
                    "No subcommand given."
                )))
            }
            if params.next().is_some() {
                for param in params {
                    if !param.is_ascii() {
                        return Err((ERR_INVALIDCAPCMD, ErrorMessage::WithSubject(
                            format!("{:?}", param), "Invalid subcommand."
                        )))
                    }
                }
                Some(1)
            } else {
                None
            }
        };
        Ok(Handler {
            msg: message,
            args: args
        })
    }
    fn invoke(self, server: &mut Server, client: Client) {
        use self::Subcommand::*;
        use user::Status::*;
        match self.subcmd() {
            LS => server.send_msg(&client, CAP, &[client.nick().as_bytes(), LS.as_bytes()]),
            LIST => server.send_msg(&client, CAP, &[client.nick().as_bytes(), LIST.as_bytes()]),
            REQ => {
                let status = {
                    // Prevent dead-lock
                    client.info().status()
                };
                match status {
                    Negotiating(_) => {},
                    NickRegistered => {
                        client.info_mut().set_status(user::STATUS_NEG_NICKREG)
                    }
                    NameRegistered => {
                        client.info_mut().set_status(user::STATUS_NEG_NAMEREG)
                    }
                    _ => {
                        client.info_mut().set_status(user::STATUS_NEG_CONNECT)
                    }
                }
                if let Some(args) = self.args {
                    server.send_msg(&client, CAP, &[client.nick().as_bytes(), NAK.as_bytes(), self.msg.params().nth(args).unwrap()])
                } else {
                    server.send_msg(&client, CAP, &[client.nick().as_bytes(), NAK.as_bytes()])
                }
            }
            END => {
                let status = {
                    // Prevent dead-lock
                    client.info().status()
                };
                match status {
                    Negotiating(&NickRegistered) => {
                        client.info_mut().set_status(NickRegistered)
                    },
                    Negotiating(&NameRegistered) => {
                        client.info_mut().set_status(NameRegistered)
                    },
                    Negotiating(&Connected) => {
                        client.info_mut().set_status(Connected)
                    },
                    Negotiating(&Registered) => {
                        {
                            client.info_mut().set_status(Registered)
                        }
                        server.register(&client)
                    },
                    Negotiating(&Disconnected) => unreachable!(),
                    _ => {}
                }
            }
            CLEAR => {
                server.send_msg(&client, CAP, &[client.nick().as_bytes(), ACK.as_bytes()])
            }
            _ => {} // ignore other commands
        }
    }
}

impl Handler {
    fn subcmd(&self) -> Subcommand {
        Subcommand::from_slice(self.msg.params().nth(0).unwrap()).unwrap()
    }
}