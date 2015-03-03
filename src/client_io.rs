//! Multi-threaded io loop
use std::collections::{VecDeque, HashMap};
use std::error::FromError;
use std::io::Cursor;
use std::io;
use std::mem;
use std::sync::{RwLock, Arc};
use std::sync::mpsc::Sender;
use std::default::Default;

use mio::{EventLoop, EventLoopSender, Handler, Token, TryRead, TryWrite, PollOpt, Interest};
use mio::net::tcp::TcpStream;
use mio::buf::{RingBuf, MutBuf, Buf};
use mio::NonBlock::*;
use mio;

use protocol::{Message, Command};
use client::{Client, ClientId, MessageOrigin};
use user::{User, Status};
use server;

pub enum Event {
    /// New TCP connection has been established
    NewConnection(TcpStream),
    /// Disconnect client
    Disconnect(ClientId),
    /// Raw message that should be send to the client as it is.
    Message(ClientId, Vec<u8>),
    /// Shared raw message that should be send to the client as it is.
    SharedMessage(ClientId, Arc<Vec<u8>>),
    /// Shut down the event loop
    Shutdown
}

/// Worker thread
pub struct Worker {
    streams: HashMap<Token, TcpStream>,
    clients: HashMap<Token, Client>,
    readers: HashMap<Token, MessageReader>,
    buffers: HashMap<Token, VecDeque<Cursor<Vec<u8>>>>,
    server_tx: EventLoopSender<server::Event>,
    host: Arc<String>

}

impl Worker {

    pub fn new(tx: EventLoopSender<server::Event>, host: Arc<String>) -> Worker {
        Worker {
            streams: HashMap::new(),
            clients: HashMap::new(),
            readers: HashMap::new(),
            buffers: HashMap::new(),
            server_tx: tx,
            host: host
        }
    }

    /// Registers a new connection
    fn register_connection(&mut self, mut stream: TcpStream, 
                           event_loop: &mut EventLoop<(), Event>) -> io::Result<ClientId>
    {
        let id = try!(ClientId::new(&stream));
        let client_hostname = ::net::get_nameinfo_mio(try!(stream.peer_addr()));
        let client = Client::new(
            id,
            User::new(client_hostname),
            event_loop.channel(),
            self.host.clone(),
        );
        let token = id.token();
        if let Ok(()) = event_loop.register_opt(
                &mut stream, token, 
                Interest::readable() | Interest::writable() | Interest::hup(), 
                PollOpt::edge()
        ) {
            self.streams.insert(token, stream);
            self.clients.insert(token, client.clone());
            self.readers.insert(token, Default::default());
            self.buffers.insert(token, VecDeque::new());
            let _ = self.server_tx.send(server::Event::Connected(client));
            Ok(id)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to register stream in event loop.",
                None
            ))
        }
    }

    fn unregister_connection(&mut self, token: Token, event_loop: &mut EventLoop<(), Event>) {
        if let Some(stream) = self.streams.remove(&token) {
            let _ = event_loop.deregister(&stream);
        } else {
            return // connection already closed
        }
        let _ = self.server_tx.send(server::Event::Disconnected(self.clients[token].clone()));
        self.readers.remove(&token);
        self.clients.remove(&token);
        self.buffers.remove(&token);
    }
}

impl Handler<(), Event> for Worker {
    fn notify(&mut self, event_loop: &mut EventLoop<(), Event>, msg: Event) {
        use self::Event::*;
        match msg {
            NewConnection(stream) => {
                // If it didn’t work the client closed the connection, never mind.
                let _ = self.register_connection(stream, event_loop);
            },
            Disconnect(id) => {
                self.unregister_connection(id.token(), event_loop);
            },
            Shutdown => {
                event_loop.shutdown()
            },
            Message(id, vec) => {
                debug!(" sending message {}", String::from_utf8_lossy(vec.as_slice()));
                if self.buffers.contains_key(&id.token()) {
                    self.buffers[id.token()].push_back(Cursor::new(vec));
                    self.writable(event_loop, id.token())
                }
            },
            SharedMessage(id, vec) => {
                debug!(" sending message {}", String::from_utf8_lossy(vec.as_slice()));
                // TODO do not clone, Cursor should also work for soon
                if self.buffers.contains_key(&id.token()) {
                    self.buffers[id.token()].push_back(Cursor::new((*vec).clone()));
                    self.writable(event_loop, id.token())
                }
            }
        }
    }
    fn readable(&mut self, event_loop: &mut EventLoop<(), Event>, token: Token, hint: mio::ReadHint) {
        use protocol::Command::*;
        if hint.is_error() || hint.is_hup() {
            if let Some(client) = self.clients.get(&token) {
                // The quit message will trigger a disconnect event
                let _ = self.server_tx.send(server::Event::InboundMessage(client.id(), Message::new(client.build_msg(
                    QUIT, &[b"Client hung up"], MessageOrigin::User
                )).unwrap()));
            }
        } else {
            if let Some(stream) = self.streams.get_mut(&token) {
                let reader = &mut self.readers[token];
                let client = &self.clients[token];
                match reader.feed(stream) {
                    Ok(messages) => for message in messages {
                        match message.map(|m| Message::new(m)) {
                            Ok(Ok(msg)) => {
                                debug!("received message {:?}", String::from_utf8_lossy(&*msg));
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
                                if let Err(_) = self.server_tx.send(server::Event::InboundMessage(client.id(), msg)) {
                                    // Server thread crashed, quitting client thread
                                    event_loop.shutdown()
                                }
                                if cmd.map_or(false, |c| c == QUIT) {
                                    // Closing connection
                                    //self.deregister_connection(token, event_loop)
                                }
                            },
                            Ok(Err(err)) => debug!("{:?}", err),
                            Err(err) => debug!("{:?}", err)
                        }
                    },
                    Err(err) => debug!("{:?}", err)
                }
            }
        }
    }
    fn writable(&mut self, _: &mut EventLoop<(), Event>, token: Token) {
        if let Some(stream) = self.streams.get_mut(&token) {
            let buffers = &mut self.buffers[token];
            while buffers.len() > 0 {
                let mut drop_front = false;
                {
                    let buffer = &mut buffers[0];
                    let max_pos = buffer.get_ref().len() as u64;
                    match stream.write_slice(&*buffer.get_ref()) {
                        Ok(WouldBlock) => break,
                        Ok(Ready(bytes)) => {
                            let new_pos = buffer.position() + bytes as u64;
                            if new_pos == max_pos {
                                drop_front = true;
                            } else {
                                buffer.set_position(new_pos)
                            }
                        },
                        Err(_) => break
                    }
                }
                if drop_front {
                    let _ = buffers.remove(0);
                }
            }
        }
    }
}

#[derive(Debug)]
enum MessageError {
    TooLong,
    Malformed,
    IoError(io::Error)
}

impl FromError<io::Error> for MessageError {
    fn from_error(err: io::Error) -> MessageError {
        MessageError::IoError(err)
    }
}

#[derive(Debug)]
struct MessageReader {
    buf: RingBuf,
    message: Vec<u8>,
    capacity: usize,
    error: bool,
    got_r: bool,
}

impl Default for MessageReader {
    fn default() -> MessageReader {
        MessageReader::new(512)
    }
}

impl MessageReader {
    fn new(capacity: usize) -> MessageReader {
        MessageReader {
            buf: RingBuf::new(capacity),
            message: Vec::with_capacity(capacity),
            capacity: capacity,
            error: false,
            got_r: false
        }
    }
    fn feed<R: TryRead>(&mut self, r: &mut R) -> io::Result<&mut MessageReader> {
        try!(r.read(&mut self.buf.writer()));
        Ok(self)
    }

    fn clear_error(&mut self) {
        let mut reader = self.buf.reader();
        let mut got_r = false;
        if self.error {
            let mut i = 0;
            for (j, &b) in reader.bytes().iter().enumerate() {
                if b == b'\r' {
                    i = j + 1;
                    got_r = true
                } else if b == b'\n' {
                    if got_r {
                        self.error = false;
                        self.got_r = false;
                        break
                    }
                } else { i = j } // skip
            }
            reader.advance(i+1); // consume bytes
        }
    }
}

impl Iterator for MessageReader {
    type Item = Result<Vec<u8>, MessageError>;

    fn next(&mut self) -> Option<Result<Vec<u8>, MessageError>> {
        use self::MessageError::*;
        self.clear_error();
        let capacity = self.capacity;
        let mut reader = self.buf.reader();
        let mut i = 0;
        let mut result = Ok(None);
        for (j, &b) in reader.bytes().iter().enumerate() {
            let res = match b {
                // Message may not include \r or \n thus now he message end is reached
                b'\r' => {
                    self.got_r = true;
                    Ok(None)
                }
                b'\n' => {
                    if self.got_r {
                        self.got_r = false;
                        Ok(Some(()))
                    } else {
                        Err(Malformed)
                    }
                }
                // This should no happen
                0 => {
                    Err(Malformed)
                }
                c => {
                    self.message.push(c);
                    if self.message.len() < capacity {
                        Ok(None)
                    } else {
                        Err(TooLong)
                    }
                }
            };
            i = j;
            result = res;
            match result {
                Ok(None) => (),
                Ok(Some(())) => break,
                Err(_) => break
            }
        }
        reader.advance(i+1); // consume bytes
        match result {
            Ok(Some(())) => {
                Some(Ok(mem::replace(&mut self.message, Vec::new())))

            },
            Ok(None) => {
                None
            },
            Err(err) => {
                self.message.clear();
                self.error = true;
                Some(Err(err))
            }
        }
    }
}