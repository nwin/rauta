use std::ops::RangeFrom;
use std::ascii::AsciiExt;
use protocol::{ResponseCode, Message};
use protocol::ResponseCode::*;
use protocol::Command::*;
use client::Client;
use server::Server;
use super::MessageHandler;

/// Handler for CAP command.
/// CAP subcommand [params]
pub struct CapHandler {
    msg: Message,
    args: Option<RangeFrom<usize>>
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
    fn as_slice(&self) -> &str {
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
    fn as_bytes(&self) -> &[u8] {
        self.as_slice().as_bytes()
    }
}

impl MessageHandler for CapHandler {
    fn from_message(message: Message) -> Result<CapHandler, (ResponseCode, String)> {
        let args = {
            let mut params = message.params();
            if let Some(ref param) = params.next() {
                if Subcommand::from_slice(param).is_none() {
                    return Err((ERR_INVALIDCAPCMD, format!(
                        "{:?} is not a valid CAP subcommand.", param
                    )))
                }
            } else {
                return Err((ERR_INVALIDCAPCMD, format!(
                    "No subcommand given."
                )))
            }
            if params.next().is_some() {
                for param in params {
                    if !param.is_ascii() {
                        return Err((ERR_INVALIDCAPCMD, format!(
                            "Arguments contain non-ASCII characters."
                        )))
                    }
                }
                Some(1..)
            } else {
                None
            }
        };
        Ok(CapHandler {
            msg: message,
            args: args
        })
    }
    fn invoke(&self, server: &mut Server, client: &mut Client) {
        match self.subcmd() {
            LS => {
                server.send_msg(client, CAP, &[Subcommand::LIST.as_bytes()])
            },
            _ => {} // ignore other commands
        }
    }
}

impl CapHandler {
    fn subcmd(&self) -> Subcommand {
        Subcommand::from_slice(self.msg.params().nth(0).unwrap()).unwrap()
    }
}