use std::old_io as io;
use std::old_io::net;
use std::old_io::{Acceptor, Listener, IoResult};
use std::ops::Deref;
use std::old_io::net::tcp::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;
use std::collections::HashMap;

use protocol::{Command, ResponseCode, Message};
use client::{ClientId, Client};
use message::handler;

pub struct Server {
    host: String,
    ip: String,
    port: u16,
    clients: HashMap<ClientId, Client>,
}

pub enum Event {
    Connected(Client),
    InboundMessage(ClientId, Message)
}

/// Irc server
impl Server {
    /// Creates a new IRC server instance.
    pub fn new(host: &str) -> IoResult<Server> {
        let addresses = try!(net::get_host_addresses(host));
        debug!("IP addresses found (for {:?}): {:?}", host, addresses);
        // Listen only on ipv4 for now…
        let ip = match addresses.iter().filter(
            |&v| match v { &net::ip::Ipv4Addr(_, _, _, _) => true, _ => false }
        ).nth(0) {
            Some(ip) => ip,
            None => return Err(io::IoError {
                kind: io::OtherIoError,
                desc: "Cannot get host IP address.",
                detail: None
            })
        };
        Ok(Server {
            host: host.to_string(),
            ip: format!("{}", ip),
            port: 6667,
            clients: HashMap::new()
        })
    }
    
    /// Starts the main loop and listens on the specified host and port.
    pub fn serve_forever(mut self) -> IoResult<Server> {
        use self::Event::{Connected, InboundMessage};
        // todo change this to a more general event dispatching loop
        for event in try!(self.listen()).1.iter() {
            match event {
                InboundMessage(id, msg) => {
                    handler::invoke(msg, &self, &self.clients[id]);
                }
                Connected(client) => {
                    let id = client.id();
                    self.clients.insert(id, client);
                }
            }
        }
        Ok(self)
    }

    fn listen(&self) -> IoResult<(Sender<Event>, Receiver<Event>)>  {
        let listener = TcpListener::bind(&*format!("{}:{}", self.ip, self.port));
        info!("started listening on {}:{} ({})", self.ip, self.port, self.host);
        let mut acceptor = try!(listener.listen());
        let (tx, rx) = channel();
        let debug_tx = tx.clone(); // This is not really needed, only for debugging and testing
        let host = Arc::new(self.host.clone());
        spawn(move || {
            for maybe_stream in acceptor.incoming() {
                match maybe_stream {
                    Err(err) => { error!("{}", err) }
                    Ok(stream) =>  match Client::listen(stream, tx.clone(), host.clone()) {
                        Ok(()) => {},
                        Err(err) => error!("{}", err)
                    }
                }
            }
        });
        Ok((debug_tx, rx))
    }

    /// Sends a response to the client
    pub fn send_response(&self, client: &Client, code: ResponseCode, payload: &[&[u8]]) {
        client.send_response(code, payload);
    }

    /// Sends a response to the client
    pub fn send_msg(&self, client: &Client, cmd: Command, payload: &[&[u8]]) {
        client.send_msg(cmd, payload);
    }
}

#[cfg(test)]
pub fn get_test_server() -> Server {
    Server {
        host: "testserver.example.com".to_string(),
        ip: "127.0.0.1".to_string(),
        port: 0,
    }
}