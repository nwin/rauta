//! Multi-threaded io loop
use std::collections::HashMap;
use std::error::FromError;
use std::io;
use std::mem;
use std::default::Default;

use mio::{EventLoop, Handler, Token, TryRead, MioResult};
use mio::net::tcp::TcpStream;
use mio::buf::{RingBuf, MutBuf, Buf};
use mio;

use client::ClientId;

use self::Event::*;

pub enum Event {
    NewConnection(TcpStream),
}

/// Worker thread
pub struct Worker {
    tokens: HashMap<Token, ClientId>, 
    streams: HashMap<ClientId, TcpStream>,
    readers: HashMap<ClientId, MessageReader>,
    counter: usize

}

impl Worker {
    /// Registers a new connection
    fn register_connection(&mut self, mut stream: TcpStream, 
                           event_loop: &mut EventLoop<(), Event>) -> io::Result<()>
    {
        let id = try!(ClientId::new_mio(&stream));
        self.counter += 1;
        let token = Token(self.counter);
        self.tokens.insert(token, id.clone());
        if let Ok(()) = event_loop.register(&mut stream, token) {
            self.streams.insert(id, stream);
            self.readers.insert(id, Default::default());
            Ok(())
        } else {
            self.counter -= 1;
            self.tokens.remove(&token);
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to register stream in event loop.",
                None
            ))
        }
    }

    fn deregister_connection(&mut self, token: Token, event_loop: &mut EventLoop<(), Event>) {
        let id = self.tokens[token];
        let _ = event_loop.deregister(&self.streams[id]);
        self.streams.remove(&id);
        self.readers.remove(&id);
        self.tokens.remove(&token);
    }
}

impl Handler<(), Event> for Worker {
    fn notify(&mut self, event_loop: &mut EventLoop<(), Event>, msg: Event) {
        match msg {
            NewConnection(stream) => {
                // If it didn’t work the client closed the connection, never mind.
                let _ = self.register_connection(stream, event_loop);
            }
        }
    }
    fn readable(&mut self, event_loop: &mut EventLoop<(), Event>, token: Token, hint: mio::ReadHint) {
        if hint.is_error() || hint.is_hup() {
            self.deregister_connection(token, event_loop)
            // TODO broadcast QUIT
        } else {
            if let Some(&id) = self.tokens.get(&token) {
                let reader = &mut self.readers[id];
                let stream = &mut self.streams[id];
                match reader.feed(stream) {
                    Ok(messages) => for message in messages {
                        match message {
                            Ok(msg) => println!("{}", String::from_utf8_lossy(&*msg)),
                            Err(err) => debug!("{:?}", err)
                        }
                    },
                    Err(err) => debug!("{:?}", err)
                }
            }
        }
    }
}

impl Default for Worker {
    fn default() -> Worker {
        Worker {
            tokens: HashMap::new(),
            streams: HashMap::new(),
            readers: HashMap::new(),
            counter: 0,

        }
    }
}

#[derive(Debug)]
enum MessageError {
    TooLong,
    Malformed,
    MioError(mio::MioError)
}

impl FromError<mio::MioError> for MessageError {
    fn from_error(err: mio::MioError) -> MessageError {
        MessageError::MioError(err)
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
    fn feed<R: TryRead>(&mut self, r: &mut R) -> mio::MioResult<&mut MessageReader> {
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