use std::fmt;

use super::Message;
use super::ResponseCode;

macro_rules! commands {
    {$(
        $ident:ident
        #[$doc:meta];
    )*} => {
/// Enumeration of all supported IRC commands (mainly RFC1459)
#[derive(Debug)]
pub enum Command {
    $(#[$doc] $ident,)*
    ///// Numeric reply codes, see `ResponseCode`
    RESPONSE(ResponseCode)
}

impl Command {
    /// Converts bytestring to Command 
    pub fn from_message(message: &Message) -> Option<Command> {
        // TODO add REPLY(...)
        $(if message.command() == stringify!($ident) { Some(Command::$ident) } else)* {
            None
        }
    }
}

impl fmt::Display for Command {
     fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            $(Command::$ident => write!(fmt, stringify!($ident)),)*
            Command::RESPONSE(code) => write!(fmt, "{:03}", code as u16)
        }
     }
}
}}

commands!{
    //PRIVMSG     #[doc = "`PRIVMSG` command"];
    //NOTICE      #[doc = "`NOTICE` command"];
    //MODE        #[doc = "`MODE` command"];
    //JOIN        #[doc = "`JOIN` command, see http://tools.ietf.org/html/rfc1459.html#section-4.2.1"];
    //PING        #[doc = "`PING` command"];
    //WHO         #[doc = "`WHO` command"];
    //NAMES       #[doc = "`NAMES` command"];
    //TOPIC       #[doc = "`TOPIC` command"];
    //PART        #[doc = "`PART` command"];
    //QUIT        #[doc = "`QUIT` command"];
    //PONG        #[doc = "`PONG` command"];
    NICK        #[doc = "NICK <nickname> [ <hopcount> ]"];
    USER        #[doc = "USER <username> <hostname> <servername> <realname>"];
    CAP         #[doc = "CAP <subcommand> [ <param> ]"];
}
