//! Message handler implementations
use protocol::{Message, Command, ResponseCode};
use server::Server;
use client::Client;

mod cap;
mod nick;
mod user;
mod quit;
mod part;
mod join;
mod invite;
mod topic;
mod mode;
mod who;
mod names;
mod privmsg;

/// Message handler trait
pub trait MessageHandler {
    /// Construct a new message handler from a message
    ///
    /// If the message is malformed an error is  returned 
    fn from_message(message: Message) -> Result<Self, (ResponseCode, ErrorMessage)>;
    /// Invokes the message handler
    ///
    /// If an error occurs an error message is send to the client
    fn invoke(self, server: &mut Server, client: Client);
}

/// Possible error messages that can be generated when constructing a message handler
pub enum ErrorMessage {
    /// Simple error message with parameter
    WithSubject(String, &'static str),
    /// Simple error message
    Plain(&'static str),
    /// Detailed error message
    Detailed(String),
    /// No error message is generated. Only used for NOTICE
    None
}

macro_rules! handle {
    {$(
        $command:ident with $handler:ty,
    )*} => {
/// Dispatches a massage to a message handler
pub fn invoke(message: Message, server: &mut Server, client: Client) {
    match message.command() {
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
                    ErrorMessage::None => ()
                }
            }
        },)*
        Some(Command::RESPONSE(_)) => (), // ignore responses from clients
        None => ()
    }
}
}}

handle!{
    PRIVMSG with self::privmsg::Handler,
    NOTICE with self::privmsg::Handler,
    JOIN with self::join::Handler,
    INVITE with self::invite::Handler,
    WHO with self::who::Handler,
    MODE with self::mode::Handler,
    TOPIC with self::topic::Handler,
    NAMES with self::names::Handler,
    PART with self::part::Handler,
    QUIT with self::quit::Handler,
    CAP with self::cap::Handler,
    NICK with self::nick::Handler,
    USER with self::user::Handler,
}