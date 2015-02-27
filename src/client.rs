//! Module containing everything related to high-level client communication
use std::ops;
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::sync::mpsc::{Sender, channel};
use std::old_io;
use std::old_io::{BufferedReader};
use std::old_io::{BufferedWriter};
use std::old_io::net::tcp::TcpStream;
use std::old_io::net::ip::IpAddr;
use std::thread::spawn;

use server;
use user::User;
use protocol::Message;
use protocol::{Command, ResponseCode};

/// Events a client can receive
#[derive(Debug)]
pub enum Event {
    /// Raw message that should be send to the client as it is.
    RawMessage(Vec<u8>)
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

/// Struct for client communication
#[derive(Clone)]
pub struct Client {
    id: ClientId,
    info: Arc<RwLock<User>>,
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
        let _ = server_tx.send(server::Event::Connected(Client {
            id: id,
            info: Arc::new(RwLock::new(User {name: "*".to_string()})),
            hostname: hostname.clone(),
            channel: msg_tx
        }));
        spawn(move || {
            // TODO: write a proper 510 char line iterator
            // as it is now it is probably very slow
            // TODO handle failures properly, send QUIT
            for line in BufferedReader::new(receiving_stream).lines() {
                match Message::new(line.unwrap()
                .trim_right().as_bytes().to_vec()) {
                    Ok(msg) => {
                        debug!("received message {}", String::from_utf8_lossy(&*msg));
                        // TODO: handle error here
                        let _ = server_tx.send(server::Event::InboundMessage(id, msg));
                    },
                    Err(_) => {}
                }
            }
        });
        spawn(move || {
            // TODO: socket timeout
            // implement when pings are send out
            // TODO handle failures properly, send QUIT
            let mut output_stream = BufferedWriter::new(stream);
            for event in rx.iter() {
                match event {
                    Event::RawMessage(msg) => {
                        debug!("sending message {}", String::from_utf8_lossy(msg.as_slice()));
                        output_stream.write_all(&*msg).unwrap();
                        output_stream.write_all(b"\r\n").unwrap();
                        output_stream.flush().unwrap();
                    }
                }
            }
        });
        Ok(())
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

    /// Getter for id
    pub fn id(&self) -> ClientId {
        self.id
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
        let ptr = get_fragment(unsafe{ &*(&guard as *const RwLockReadGuard<'a, T>) });
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
