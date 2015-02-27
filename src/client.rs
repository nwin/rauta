//! Module containing everything related to high-level client communication
use std::ops;
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::sync::mpsc::{Sender};
use std::old_io::net::tcp::TcpStream;

use server;
use user::User;
use protocol::Message as IrcMessage;
use protocol::{Command, ResponseCode};

/// Events a client can receive
pub enum Event {
    /// Raw message that should be send to the client as it is.
    RawMessage(Vec<u8>)
}

pub struct ClientId;

/// Struct for client communication
#[derive(Clone)]
pub struct Client {
    info: Arc<RwLock<User>>,
    hostname: Arc<String>,
    channel: Sender<Event>, 
}

impl Client {
    /// Initialized client communication
    pub fn communicate(stream: TcpStream, server_tx: Sender<server::Event>, hostname: Arc<String>) {
    }
    
    pub fn send_response(&self, code: ResponseCode, payload: &[&[u8]]) {
        self.send_msg(Command::RESPONSE(code), payload);
    }
    
    pub fn send_msg(&self, cmd: Command, payload: &[&[u8]]) {
        let mut msg = format!(":{prefix} {cmd} {user}", 
                              prefix=&*self.hostname,
                              cmd=cmd,
                              user=&*self.name()
        ).into_bytes();
        if payload.len() > 0 {
            let last = payload.len() - 1;
            for item in payload[..last].iter() {
                msg.push(b' ');
                msg.push_all(item)
            }
            msg.push_all(b" :");
            msg.push_all(payload[last])
        }
        self.send_raw(msg);
    }
    
    #[inline(always)]
    fn info(&self) -> RwLockReadGuard<User> {
        (match (*self.info).read() {
            Ok(guard) => guard,
            Err(err) => err.into_inner()
        })
    }

    /// Getter for user name
    pub fn name(&self) -> FragmentReadGuard<User, str> {
        FragmentReadGuard::new(self.info(), |g| &*g.name)
    }

    /// Sends an event to the client
    pub fn send(&self, evt: Event) {
        let _ = self.channel.send(evt);
    }

    /// Sends a raw message to the client
    pub fn send_raw(&self, msg: Vec<u8>) {
        self.send(Event::RawMessage(msg));
    }
}

macro_rules! guard {
    ($val:expr) => {
        (match (*$val).read() {
            Ok(guard) => guard,
            Err(err) => err.into_inner()
        })
    }
}

struct FragmentReadGuard<'a, T: 'a, R: ?Sized + 'a> {
    guard: RwLockReadGuard<'a, T>,
    ptr: &'a R
}

impl<'a, T, R: ?Sized> FragmentReadGuard<'a, T, R> {
    #[inline]
    fn new<F>(guard: RwLockReadGuard<'a, T>, get_fragment: F)
             -> FragmentReadGuard<'a, T, R>
             where F: Fn(&'a RwLockReadGuard<'a, T>) -> &'a R
    {
        let g = &guard as *const RwLockReadGuard<'a, T>;
        FragmentReadGuard {
            guard: guard,
            ptr: get_fragment(unsafe{ &*g }),
        }
    }
}

impl<'a, T, R: ?Sized> ops::Deref for FragmentReadGuard<'a, T, R> {
    type Target = R;
    fn deref(&self) -> &R {
        self.ptr
    }
}
