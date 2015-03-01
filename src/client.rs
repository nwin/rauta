//! Module containing everything related to high-level client communication
use std::ops;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::mpsc::{Sender, channel};
use std::old_io;
use std::old_io::{BufferedReader};
use std::old_io::{BufferedWriter};
use std::old_io::net::tcp::TcpStream;
use std::old_io::net::ip::IpAddr;
use std::thread::spawn;

use server;
use user::{User, Status};
use protocol::Message;
use protocol::{Command, ResponseCode};

/// Events a client can receive
#[derive(Debug)]
pub enum Event {
    /// Raw message that should be send to the client as it is.
    Message(Vec<u8>),
    /// Shared raw message that should be send to the client as it is.
    SharedMessage(Arc<Vec<u8>>)
}

#[derive(Hash, Copy, PartialEq, Eq, Clone, Debug)]
pub struct ClientId {
    id: [u64; 2]
}

impl ClientId {
    /// The client id is losely inspired by SILC but the silc
    /// method of also using the nickname for this is not applicable to IRC
    fn new(stream: &mut TcpStream) -> ClientId {
        ClientId { 
            id: [
                (match stream.socket_name().unwrap().ip {
                    IpAddr::Ipv4Addr(a, b, c, d) => (a as u32) <<24 | (b as u32)<<16 | (c as u32)<<8 | d as u32,
                    IpAddr::Ipv6Addr(_, _, _, _, _, _, a, b) => (a as u32) << 16 | b as u32 
                } as u64) << 32
                | match stream.peer_name().unwrap().ip {
                    IpAddr::Ipv4Addr(a, b, c, d) => (a as u32) <<24 | (b as u32) <<16 | (c as u32) <<8 | d as u32,
                    IpAddr::Ipv6Addr(_, _, _, _, _, _, a, b) => (a as u32)  << 16 | b as u32  
                } as u64, 
                42//random()
            ]
        }
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
    pub info: Arc<RwLock<User>>,
    hostname: Arc<String>,
    channel: Sender<Event>, 
}

impl Client {
    /// Initialized client communication
    pub fn listen(mut stream: TcpStream, server_tx: Sender<server::Event>, hostname: Arc<String>) -> old_io::IoResult<()> {
        let (msg_tx, rx) = channel();
        //let err_tx = msg_tx.clone();
        let peer_name = try!(stream.peer_name());
        let id = ClientId::new(&mut stream);
        let client_hostname = ::net::get_nameinfo(peer_name);
        debug!("hostname of client is {}", client_hostname.clone());
        let receiving_stream = stream.clone();
        // this has to be sended first otherwise we have a nice race conditions
        let client = Client {
            id: id,
            info: Arc::new(RwLock::new(User::new(client_hostname))),
            hostname: hostname.clone(),
            channel: msg_tx
        };
        let _ = server_tx.send(server::Event::Connected(client.clone()));
        spawn(move || {
            use protocol::Command::{CAP, NICK, USER, QUIT};
            // TODO: write a proper 510 char line iterator
            // as it is now it is probably very slow
            // TODO handle failures properly, send QUIT
            let mut line_reader = BufferedReader::new(receiving_stream);
            for line in line_reader.lines() {
                match line.map(|l| Message::new(l.trim_right().as_bytes().to_vec())) {
                    Ok(Ok(msg)) => {
                        debug!("received message {}", String::from_utf8_lossy(&*msg));
                        let cmd = Command::from_message(&msg);
                        if client.info().status() != Status::Registered {
                            match cmd {
                                Some(CAP) | Some(NICK) | Some(USER) | Some(QUIT) => (),
                                Some(cmd) => {
                                    // User is not registered, ignore other messages for now
                                    debug!("User not yet registered ignored {} message.", cmd);
                                    continue
                                }
                                _ => ()
                            }
                        }
                        if let Err(_) = server_tx.send(server::Event::InboundMessage(id, msg)) {
                            // Server thread crashed, quitting client thread
                            break
                        }
                        if cmd.map_or(false, |c| c == QUIT) {
                            // Closing connection
                            break
                        }
                    },
                    Ok(Err(_)) => {} // TODO send error reason
                    Err(_) => {} // TODO close connection and send quit message
                }
            }
            let mut stream = line_reader.into_inner();
            let _ = stream.close_read();
            let _ = stream.close_write();
        });
        spawn(move || {
            use self::Event::*;
            // TODO: socket timeout
            // implement when pings are send out
            // TODO handle failures properly, send QUIT
            let mut output_stream = BufferedWriter::new(stream);
            for event in rx.iter() {
                match event {
                    Message(msg) => {
                        debug!(" sending message {}", String::from_utf8_lossy(msg.as_slice()));
                        output_stream.write_all(&*msg).unwrap();
                        output_stream.write_all(b"\r\n").unwrap();
                        output_stream.flush().unwrap();
                    }
                    SharedMessage(msg) => {
                        let msg = &*msg;
                        debug!(" sending message {}", String::from_utf8_lossy(msg.as_slice()));
                        output_stream.write_all(&*msg).unwrap();
                        output_stream.write_all(b"\r\n").unwrap();
                        output_stream.flush().unwrap();
                    }
                }
            }
        });
        Ok(())
    }

    fn push_tail(&self, mut msg: Vec<u8>, payload: &[&[u8]]) -> Vec<u8> {
        if payload.len() > 0 {
            let last = payload.len() - 1;
            for item in payload[..last].iter() {
                msg.push(b' ');
                msg.push_all(item)
            }
            msg.push_all(b" :");
            msg.push_all(payload[last])
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
    
    /// Builds a raw message
    pub fn build_msg(&self, cmd: Command, payload: &[&[u8]], origin: MessageOrigin) -> Vec<u8> {
        use self::MessageOrigin::*;

        let msg = match origin { 
            Server => format!(":{prefix} {cmd}", prefix=&*self.hostname, cmd=cmd),
            User => format!(":{prefix} {cmd}", prefix=&*self.nick(), cmd=cmd),
        }.into_bytes();
        self.push_tail(msg, payload)
    }
    
    /// Sends a message to the client
    pub fn send_msg(&self, cmd: Command, payload: &[&[u8]], origin: MessageOrigin) {
        self.send_raw(self.build_msg(cmd, payload, origin));
    }
    
    /// Sends a response to the client
    pub fn send_response(&self, code: ResponseCode, payload: &[&str]) {
        self.send_raw(self.build_response(code, payload));
    }

    /// Sends an event to the client
    pub fn send(&self, evt: Event) {
        // TODO handle error
        let _ = self.channel.send(evt);
    }

    /// Sends a raw message to the client
    pub fn send_raw(&self, msg: Vec<u8>) {
        self.send(Event::Message(msg));
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
    pub fn nick(&self) -> FragmentReadGuard<User, str> {
        FragmentReadGuard::new(self.info(), |g| g.nick())
    }

    /// Getter for host name
    pub fn hostname(&self) -> &Arc<String> {
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

/// A wrapped RwLockReadGuard that only allows access to a part of the guarded
/// struct
pub struct FragmentReadGuard<'a, T: 'a, R: ?Sized + 'a> {
    guard: RwLockReadGuard<'a, T>,
    ptr: &'a R
}

impl<'a, T, R: ?Sized> FragmentReadGuard<'a, T, R> {
    #[inline]
    fn new<F>(guard: RwLockReadGuard<'a, T>, get_fragment: F)
             -> FragmentReadGuard<'a, T, R>
             where F: Fn(&'a RwLockReadGuard<'a, T>) -> &'a R
    {

        // This works because ptr is not a reference into the guard
        // but into the guarded object. Thus moving the guard does not
        // invalidate the pointer.
        let ptr = get_fragment(
            unsafe{ &*(&guard as *const RwLockReadGuard<'a, T>) }
        );
        FragmentReadGuard {
            guard: guard,
            ptr: ptr,
        }
    }
}

impl<'a, T, R: ?Sized> ops::Deref for FragmentReadGuard<'a, T, R> {
    type Target = R;
    fn deref(&self) -> &R {
        self.ptr
    }
}