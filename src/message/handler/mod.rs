//! Message handler implementations

use protocol::{Message, Command, ResponseCode};
use server::Server;
use client::Client;

mod cap;

pub trait MessageHandler {
    fn from_message(message: Message) -> Result<Self, (ResponseCode, String)>;
    fn invoke(&self, server: &Server, client: &Client);
}

macro_rules! handle {
    {$(
        $command:ident with $handler:path,
    )*} => {
/// Temporary dispatcher
pub fn invoke(message: Message, server: &Server, client: &Client) -> Result<(), (ResponseCode, String)> {
    match Command::from_message(&message) {
        $(Some(Command::$command) => {
        	let res: Result<$handler, (ResponseCode, String)> = MessageHandler::from_message(message);
        	res.map( |handler| {
        		handler.invoke(server, client);
        	})
        },)*
        Some(Command::RESPONSE(_)) => Ok(()), // ignore responses from clients
        None => Ok(())
    }
}
}}

handle!{
    CAP with self::cap::Handler,
}