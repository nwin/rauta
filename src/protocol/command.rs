use std::fmt;

use super::Message;
use super::ResponseCode;

macro_rules! commands {
    {$(
        $ident:ident
        #[$doc:meta];
    )*} => {
/// Enumeration of all supported IRC commands (mainly RFC1459)
#[derive(Debug, PartialEq)]
pub enum Command {
    $(#[$doc] $ident,)*
    /// Numeric reply codes, see `ResponseCode`
    RESPONSE(ResponseCode)
}

impl Command {
    /// Extracts the command from a Message. 
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
    PRIVMSG     #[doc = "`PRIVMSG <msgtarget> <text to be sent>`"];
    //NOTICE      #[doc = "`NOTICE` command"];
    MODE        #[doc = "`MODE <channel> {[+|-]|o|p|s|i|t|n|b|v} [<limit>] [<user>] [<ban mask>]`"];
    JOIN        #[doc = "`JOIN ( <channel> *( \",\" <channel> ) [ <key> *( \",\" <key> ) ] )/ \"0\"`"];
    //PING        #[doc = "`PING` command"];
    WHO         #[doc = "`WHO [ <mask> [ \"o\" ] ]`"];
    NAMES       #[doc = "`NAMES [ <channel> *( \",\" <channel> ) [ <target> ] ]`"];
    TOPIC       #[doc = "`TOPIC <channel> [ <topic> ]`"];
    PART        #[doc = "`PART <channel> *( \",\" <channel> ) [ <Part Message> ]`"];
    QUIT        #[doc = "`QUIT [<reason>]`"];
    //PONG        #[doc = "`PONG` command"];
    NICK        #[doc = "`NICK <nickname> [ <hopcount> ]`"];
    USER        #[doc = "`USER <username> <hostname> <servername> <realname>`"];
    CAP         #[doc = "`CAP <subcommand> [ <param> ]`"];
}
