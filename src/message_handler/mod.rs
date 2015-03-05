//! Message handler implementations
use std::ops::Range;
use std::str;
use std::marker::PhantomData;

use protocol::{Message, Command, ResponseCode};
use protocol::Params;
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

pub enum ParseError<'a> {
    Missing,
    TooMany,
    Malformed(&'a [u8]),
}

enum OnError {
    Skip,
    Fail
}

#[derive(Debug)]
/// Parses a and verifies a comma separated list
pub struct CommaSeparated<T: ?Sized> {
    index: usize,
    parameters: [Range<usize>; 10],
    _phantom: PhantomData<Box<T>>
}

impl<T: ?Sized> CommaSeparated<T> {
    fn verify<'a, F>(verify: F, params: Params<'a>, index: usize)
    -> Result<CommaSeparated<T>, ParseError<'a>> 
    where F: Fn(&[u8]) -> Option<&T> {
        CommaSeparated::verify_on_error(verify, params, index, OnError::Fail)
    }
    fn verify_no_error<'a, F>(verify: F, params: Params<'a>, index: usize)
    -> CommaSeparated<T>
    where F: Fn(&[u8]) -> Option<&T>  {
        CommaSeparated::verify_on_error(verify, params, index, OnError::Skip).ok().unwrap()
    }
    /// If called with on_error = Skip the result is safe to unwrap
    fn verify_on_error<'a, F>(verify: F, mut params: Params<'a>, index: usize, on_error: OnError)
    -> Result<CommaSeparated<T>, ParseError<'a>>
    where F: Fn(&[u8]) -> Option<&T> {
        let mut parameters = [0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0];
        if let Some(params) = params.nth(index) {
            let mut start = 0;
            let mut i = 0;
            for param in params.split(|c| *c == b',') {
                let len = param.len();
                if len > parameters.len() { match on_error {
                        OnError::Skip => (),
                        OnError::Fail => return Err(ParseError::TooMany)
                }}
                match verify(param) {
                    Some(_) => {
                        parameters[i] = start..start+len;
                        i += 1;
                    },
                    None => match on_error {
                        OnError::Skip => (),
                        OnError::Fail => return Err(ParseError::Malformed(param))
                    }
                }
                start += len + 1
            }
            Ok(CommaSeparated {
                index: index,
                parameters: parameters,
                _phantom: PhantomData
            })
        } else {
            Err(ParseError::Missing)
        }
    }

    /// Generates an iterator over the parameters
    fn iter<'a>(&'a self, mut params: Params<'a>) -> ParameterIterator<'a, T> {
        ParameterIterator {
            list: self,
            params: params.nth(self.index).unwrap(),
            pos: 0,
        }
    }

    fn empty() -> CommaSeparated<T> {
        CommaSeparated {
            index: 0,
            parameters: [0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0, 0..0],
            _phantom: PhantomData
        }
    }
}

#[derive(Debug)]
pub struct ParameterIterator<'a, T: ?Sized> where T: 'a {
    list: &'a CommaSeparated<T>,
    params: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for ParameterIterator<'a, str> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        use std::mem::transmute;
        unsafe {
            transmute::<_, &mut ParameterIterator<'a, [u8]>>(self).next().map(
                |v| str::from_utf8(v).unwrap()
                // possible speed optimization:
                // if uncommented iter() must be marked as unsafe and called with the
                // same arguments as verify
                //|v| transmute(v)
            )
        }
        

    }
}

impl<'a> Iterator for ParameterIterator<'a, [u8]> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        use std::mem;
        if let Some(item) = self.list.parameters.get(self.pos) {
            if item != &(0..0) {
                let res = &self.params[*item];
                self.pos += 1;
                Some(res)
            } else {
                None
            }
        } else {
            None
        }

    }
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