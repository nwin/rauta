//! Message handler implementations
use std::ops;

use protocol::{Message, Command, ResponseCode};
use server::Server;
use client::Client;

mod cap;
mod nick;
mod user;

pub trait MessageHandler {
    fn from_message(message: Message) -> Result<Self, (ResponseCode, ErrorMessage)>;
    fn invoke(self, server: &mut Server, client: Client);
}

pub enum ErrorMessage {
    WithSubject(String, &'static str),
    Plain(&'static str),
    Detailed(String)
}

macro_rules! handle {
    {$(
        $command:ident with $handler:ty,
    )*} => {
/// Temporary dispatcher
pub fn invoke(message: Message, server: &mut Server, client: Client) {
    match Command::from_message(&message) {
        $(Some(Command::$command) => {
            match <$handler>::from_message(message) {
                Ok(handler) => handler.invoke(server, client),
                Err((code, msg)) => match msg {
                    ErrorMessage::WithSubject(string, str_) => {
                        server.send_response(&client, code, &[&*string, str_])
                    },
                    ErrorMessage::Plain(str_) => {
                        server.send_response(&client, code, &[str_])
                    },
                    ErrorMessage::Detailed(string) => {
                        server.send_response(&client, code, &[&*string])
                    }
                }
            }
        },)*
        Some(Command::RESPONSE(_)) => (), // ignore responses from clients
        None => ()
    }
}
}}

handle!{
    CAP with self::cap::Handler,
    NICK with self::nick::Handler,
    USER with self::user::Handler,
}