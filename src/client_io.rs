//! Multi-threaded io loop
use std::io::prelude::*;

use std::collections::{VecDeque, HashMap};
use std::convert::From;
use std::io::Cursor;
use std::io;
use std::mem;
use std::sync::Arc;
use std::default::Default;

use mio::{self, EventLoop, Handler, Token, TryRead, TryWrite, PollOpt, EventSet};
use mio::tcp::TcpStream;
use bytes::RingBuf;

use protocol::{Message, Command};
use protocol::ResponseCode::*;
use client::{Client, ClientId, MessageOrigin};
use user::{User, Status};
use server;

/// Events that can be sent to `Worker`
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

/// Event handler for client communication
pub struct Worker {
    streams: HashMap<Token, TcpStream>,
    clients: HashMap<Token, Client>,
    readers: HashMap<Token, MessageReader>,
    buffers: HashMap<Token, VecDeque<Cursor<Vec<u8>>>>,
    server_tx: mio::Sender<server::Event>,
    host: Arc<String>

}

impl Worker {
    /// Constructs a new worker
    pub fn new(tx: mio::Sender<server::Event>, host: Arc<String>) -> Worker {
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
                           event_loop: &mut EventLoop<Worker>) -> io::Result<ClientId>
    {
        let id = try!(ClientId::new(&stream));
        let client_hostname = ::net::get_nameinfo(try!(stream.peer_addr()));
        let client = Client::new(
            id,
            User::new(client_hostname),
            event_loop.channel(),
            self.host.clone(),
        );
        let token = id.token();
        if let Ok(()) = event_loop.register(
                &mut stream, token, 
                EventSet::readable() | EventSet::writable() | EventSet::hup(), 
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
                "Failed to register stream in event loop."
            ))
        }
    }

    fn unregister_connection(&mut self, token: &Token, event_loop: &mut EventLoop<Worker>) {
        if let Some(stream) = self.streams.remove(token) {
            let _ = event_loop.deregister(&stream);
        } else {
            return // connection already closed
        }
        let _ = self.server_tx.send(server::Event::Disconnected(self.clients[token].clone()));
        self.readers.remove(token);
        self.clients.remove(token);
        self.buffers.remove(token);
    }
    
    fn readable(&mut self, event_loop: &mut EventLoop<Worker>, token: Token, events: mio::EventSet) {
        use protocol::Command::*;
        if events.is_error() || events.is_hup() {
            if let Some(client) = self.clients.get(&token) {
                // The quit message will trigger a disconnect event
                let _ = self.server_tx.send(server::Event::InboundMessage(client.id(), Message::new(client.build_msg(
                    QUIT, &["Client hung up"], MessageOrigin::User
                )).unwrap()));
            }
        } else {
            if let Some(stream) = self.streams.get_mut(&token) {
                let reader = &mut self.readers.get_mut(&token).unwrap();
                let client = &self.clients[&token];
                match reader.feed(stream) {
                    Ok(messages) => for message in messages {
                        match message.map(|m| Message::new(m)) {
                            Ok(Ok(msg)) => {
                                debug!("received message {:?}", String::from_utf8_lossy(&*msg));
                                if let Some(cmd) = msg.command() {
                                    if client.info().status() != Status::Registered {
                                        match cmd {
                                            CAP | NICK | USER | QUIT => (),
                                            cmd => {
                                                // User is not registered, ignore other messages for now
                                                debug!("User not yet registered ignored {} message.", cmd);
                                                continue
                                            }
                                        }
                                    }
                                    if let Err(_) = self.server_tx.send(server::Event::InboundMessage(client.id(), msg)) {
                                        // Server thread crashed, quitting client thread
                                        event_loop.shutdown()
                                    }
                                } else {
                                    client.send_response(
                                        ERR_UNKNOWNCOMMAND, 
                                        &[&*String::from_utf8_lossy(msg.command_bytes()), "Unknown command"]
                                    )
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
    
    fn writable(&mut self, _: &mut EventLoop<Worker>, token: Token) {
        if let Some(stream) = self.streams.get_mut(&token) {
            let buffers = &mut self.buffers.get_mut(&token).unwrap();
            while buffers.len() > 0 {
                let mut drop_front = false;
                {
                    let buffer = &mut buffers[0];
                    let max_pos = buffer.get_ref().len() as u64;
                    match stream.write(&*buffer.get_ref()) {
                        Ok(bytes) => {
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

impl Handler for Worker {
    type Timeout = ();
    type Message = Event;

    fn notify(&mut self, event_loop: &mut EventLoop<Worker>, msg: Event) {
        use self::Event::*;
        match msg {
            NewConnection(stream) => {
                // If it didn’t work the client closed the connection, never mind.
                let _ = self.register_connection(stream, event_loop);
            },
            Disconnect(id) => {
                self.unregister_connection(&id.token(), event_loop);
            },
            Shutdown => {
                event_loop.shutdown()
            },
            Message(id, vec) => {
                debug!(" sending message {}", String::from_utf8_lossy(&vec));
                let token = id.token();
                if self.buffers.contains_key(&token) {
                    self.buffers.get_mut(&token).unwrap().push_back(Cursor::new(vec));
                    self.writable(event_loop, token)
                }
            },
            SharedMessage(id, vec) => {
                debug!(" sending message {}", String::from_utf8_lossy(&vec));
                // TODO do not clone, Cursor should also work for soon
                let token = id.token();
                if self.buffers.contains_key(&token) {
                    self.buffers.get_mut(&token).unwrap().push_back(Cursor::new((*vec).clone()));
                    self.writable(event_loop, token)
                }
            }
        }
    }
    
    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        if events.is_writable() {
            self.writable(event_loop, token)
        } else if events.is_readable() {
            self.readable(event_loop, token, events)
        }
    }
}

#[derive(Debug)]
enum MessageError {
    MessageTooLong,
    MalformedMessage,
    IoError(io::Error)
}

impl From<io::Error> for MessageError {
    fn from(err: io::Error) -> MessageError {
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

/// Reads IRC messages from a stream
///
/// Ensures that the message does not exceed 512 bytes.
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

    /// Tries to re-fill the internal buffer
    ///
    /// The returns MessageReader is an Interator over the messages that can be 
    /// reconstructed from the internal buffer.
    pub fn feed<R: Read>(&mut self, r: &mut R) -> io::Result<&mut MessageReader> {
        use bytes::MutBuf;
        let n_bytes = try!(r.read(unsafe {&mut self.buf.mut_bytes()}));
        unsafe { self.buf.advance(n_bytes) };
        Ok(self)
    }

    /// Resets the internal error state
    ///
    /// If the reader is in an error state all characters are skipped until
    /// a message separator is found ("\r\n")
    fn clear_error(&mut self) {
        use bytes::Buf;
        let reader = &mut self.buf;
        let mut got_r = false;
        if self.error {
            let mut i = 0;
            for (j, &b) in (reader as &mut Buf).bytes().iter().enumerate() {
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
        use bytes::Buf;
        use self::MessageError::*;
        self.clear_error();
        let capacity = self.capacity;
        let mut reader = &mut self.buf;
        let mut i = 0;
        let mut result = Ok(None);
        for (j, &b) in (reader as &mut Buf).bytes().iter().enumerate() {
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
                        Err(MalformedMessage)
                    }
                }
                // This should no happen
                0 => {
                    Err(MalformedMessage)
                }
                c => {
                    self.message.push(c);
                    if self.message.len() < capacity {
                        Ok(None)
                    } else {
                        Err(MessageTooLong)
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