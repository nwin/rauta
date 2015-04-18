//! High-level client communication
use std::ops;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use mio;
use std::io;
use std::net;
use std::mem;

use rand;

use client_io;
use user::User;
use protocol::{Command, ResponseCode};

#[derive(Hash, Copy, PartialEq, Eq, Clone, Debug)]
pub struct ClientId {
    id: [u64; 2]
}

impl ClientId {
    /// The client id is losely inspired by SILC but the silc
    /// method of also using the nickname for this is not applicable to IRC
    pub fn new(stream: &mio::tcp::TcpStream) -> io::Result<ClientId> {
        Ok(ClientId { 
            id: [
                (match try!(stream.local_addr()) {
                    net::SocketAddr::V4(addr) => {
                        let [a, b, c, d] = addr.ip().octets();
                        (a as u32) <<24 | (b as u32)<<16 | (c as u32)<<8 | d as u32
                    },
                    net::SocketAddr::V6(addr) => {
                        let [_, _, _, _, _, _, a, b] = addr.ip().segments();
                        (a as u32)  << 16 | b as u32
                    },
                } as u64) << 32
                | match try!(stream.peer_addr()) {
                    net::SocketAddr::V4(addr) => {
                        let [a, b, c, d] = addr.ip().octets();
                        (a as u32) <<24 | (b as u32)<<16 | (c as u32)<<8 | d as u32
                    },
                    net::SocketAddr::V6(addr) => {
                        let [_, _, _, _, _, _, a, b] = addr.ip().segments();
                        (a as u32)  << 16 | b as u32
                    },
                } as u64, 
                rand::random()
            ]
        })
    }

    pub fn token(&self) -> mio::Token {
        mio::Token((self.id[0] ^ self.id[1]) as usize)
    }
}

/// Origin of a message
pub enum MessageOrigin {
    Server,
    User
}

/// Struct for client communication
#[derive(Clone)]
pub struct Client {
    id: ClientId,
    info: Arc<RwLock<User>>,
    hostname: Arc<String>,
    channel: mio::Sender<client_io::Event>, 
}

impl Client {
    /// Initialized client communication
    pub fn new(id: ClientId, user: User, tx: mio::Sender<client_io::Event>, hostname: Arc<String>) -> Client {
        Client {
            id: id,
            info: Arc::new(RwLock::new(user)),
            hostname: hostname,
            channel: tx
        }
    }

    fn push_tail(&self, mut msg: Vec<u8>, payload: &[&[u8]]) -> Vec<u8> {
        if payload.len() > 0 {
            let last = payload.len() - 1;
            for item in payload[..last].iter() {
                msg.push(b' ');
                msg.push_all(item)
            }
            msg.push_all(b" :");
            msg.push_all(payload[last]);
            msg.push_all(b"\r\n");
        }
        msg
    }
    
    /// Builds a raw response message
    pub fn build_response(&self, code: ResponseCode, payload: &[&str]) -> Vec<u8> {
        use std::mem;
        let msg = format!(":{prefix} {cmd} {user}", 
                          prefix=&*self.hostname,
                          cmd=Command::RESPONSE(code),
                          user=&*self.nick()
        ).into_bytes();
        // Unfortunately there is no other way to efficiently convert &[&str] to &[&[u8]]
        self.push_tail(msg, unsafe { mem::transmute(payload) })
    }
    
    /// Builds a message of behalf of this client
    pub fn build_msg(&self, cmd: Command, payload: &[&str], origin: MessageOrigin) -> Vec<u8> {
        self.build_raw_msg(cmd, unsafe {mem::transmute(payload)}, origin)
    }
    
    /// Builds a raw message of behalf of this client
    pub fn build_raw_msg(&self, cmd: Command, payload: &[&[u8]], origin: MessageOrigin) -> Vec<u8> {
        use self::MessageOrigin::*;

        let msg = match origin { 
            Server => format!(":{prefix} {cmd}", prefix=&*self.hostname, cmd=cmd),
            //User => format!(":{prefix} {cmd}", prefix=&*self.nick(), cmd=cmd),
            User => format!(":{mask} {cmd}", 
                mask=self.info().public_hostmask().as_str(),
                cmd=cmd),
        }.into_bytes();
        self.push_tail(msg, payload)
    }
    
    /// Sends a message to the client
    pub fn send_msg(&self, cmd: Command, payload: &[&str], origin: MessageOrigin) {
        self.send_raw(self.build_msg(cmd, payload, origin));
    }
    
    /// Sends a message to the client
    pub fn send_raw_msg(&self, cmd: Command, payload: &[&[u8]], origin: MessageOrigin) {
        self.send_raw(self.build_raw_msg(cmd, payload, origin));
    }
    
    /// Sends a message on behalf of `origin` to the client
    pub fn send_msg_from(&self, cmd: Command, payload: &[&str], origin: &Client) {
        self.send_raw_msg_from(cmd, unsafe {mem::transmute(payload)}, origin);
    }
    
    /// Sends a raw message on behalf of `origin` to the client
    pub fn send_raw_msg_from(&self, cmd: Command, payload: &[&[u8]], origin: &Client) {
        self.send_raw(origin.build_raw_msg(cmd, payload, MessageOrigin::User));
    }
    
    /// Sends a response to the client
    pub fn send_response(&self, code: ResponseCode, payload: &[&str]) {
        self.send_raw(self.build_response(code, payload));
    }

    /// Sends an event to the client
    pub fn send(&self, evt: client_io::Event) {
        // TODO handle error
        let _ = self.channel.send(evt);
    }

    /// Sends a raw message to the client
    pub fn send_raw(&self, msg: Vec<u8>) {
        self.send(client_io::Event::Message(self.id(), msg));
    }
    
    /// Getter for info
    #[inline(always)]
    pub fn info(&self) -> RwLockReadGuard<User> {
        (match (*self.info).read() {
            Ok(guard) => guard,
            Err(err) => err.into_inner()
        })
    }
    
    /// Mut getter for info
    #[inline(always)]
    pub fn info_mut(&self) -> RwLockWriteGuard<User> {
        self.info.write().unwrap()
    }

    /// Getter for id
    pub fn id(&self) -> ClientId {
        self.id
    }

    /// Getter for user name
    pub fn nick(&self) -> ReadGuard<User, str> {
        ReadGuard::new(self.info(), |info| info.nick())
    }

    /// Getter for server name
    pub fn server_name(&self) -> &Arc<String> {
        &self.hostname
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

/// Wraps a RwLockReadGuard
///
/// Allows to expose only a part of the guarded struct.
pub struct ReadGuard<'a, T: 'a, R: ?Sized + 'a> {
    _guard: RwLockReadGuard<'a, T>,
    ptr: &'a R
}

impl<'a, T, R: ?Sized> ReadGuard<'a, T, R> {
    #[inline]
    fn new<F>(guard: RwLockReadGuard<'a, T>, do_expose: F)
             -> ReadGuard<'a, T, R>
             where F: Fn(&'a RwLockReadGuard<'a, T>) -> &'a R
    {

        // This works because ptr is not a reference into the guard
        // but into the guarded object. Thus moving the guard does not
        // invalidate the pointer.
        let ptr = do_expose(
            unsafe{ &*(&guard as *const RwLockReadGuard<'a, T>) }
        );
        ReadGuard {
            _guard: guard,
            ptr: ptr,
        }
    }
}

impl<'a, T, R: ?Sized> ops::Deref for ReadGuard<'a, T, R> {
    type Target = R;
    fn deref(&self) -> &R {
        self.ptr
    }
}