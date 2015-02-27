//! Message handler implementations

use protocol::{Message, ResponseCode};
use server::Server;
use client::Client;

mod cap;

pub trait MessageHandler {
    fn from_message(message: Message) -> Result<Self, (ResponseCode, String)>;
    fn invoke(&self, server: &mut Server, client: &mut Client);
}
