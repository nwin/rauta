use std::ascii::AsciiExt;
use std::ops::Deref;
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

impl Deref for Subcommand {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_slice()
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
        // TODO CAP LS stop also client registration!!
        use self::Subcommand::*;
        match self.subcmd() {
            LS => {
                suspend_registration(&client);
                server.send_msg(&client, CAP, &[&*client.nick(), &*LS])
            },
            LIST => server.send_msg(&client, CAP, &[&*client.nick(), &*LIST]),
            REQ => {
                suspend_registration(&client);
                if let Some(args) = self.args {
                    server.send_raw_msg(&client, CAP, &[client.nick().as_bytes(), NAK.as_bytes(), self.msg.params().nth(args).unwrap()])
                } else {
                    server.send_msg(&client, CAP, &[&*client.nick(), &*NAK])
                }
            }
            END => {
                if continue_registration(&client) {
                    server.register(&client)
                }
            }
            CLEAR => {
                server.send_msg(&client, CAP, &[&*client.nick(), &*ACK])
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

/// Suspends the registration process
fn suspend_registration(client: &Client) {
    use user::Status::*;
    let status: user::Status = {
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
}

/// Un-suspends the registration process
///
/// Returns true if the client should be registered now
fn continue_registration(client: &Client) -> bool {
    use user::Status::*;
    let status: user::Status = {
        // Prevent dead-lock
        client.info().status()
    };
    match status {
        Negotiating(&NickRegistered) => {
            client.info_mut().set_status(NickRegistered);
            false
        },
        Negotiating(&NameRegistered) => {
            client.info_mut().set_status(NameRegistered);
            false
        },
        Negotiating(&Connected) => {
            client.info_mut().set_status(Connected);
            false
        },
        Negotiating(&Registered) => {
            {
                client.info_mut().set_status(Registered)
            }
            true
        },
        Negotiating(&Disconnected) => unreachable!(),
        _ => false
    }
}