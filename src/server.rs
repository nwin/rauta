//! Server model

use std::old_io as io;
use std::old_io::net;
use std::old_io::{Acceptor, Listener, IoResult};
use std::old_io::net::tcp::{TcpListener};
use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;
use std::collections::HashMap;

use protocol::{Command, ResponseCode, Message};
use client::{ClientId, Client, MessageOrigin};
use message_handler;
use channel;

pub struct Server {
    host: String,
    ip: String,
    port: u16,
    clients: HashMap<ClientId, Client>,
    nicks: HashMap<String, ClientId>,
    channels: HashMap<String, channel::Proxy>,
    tx: Option<Sender<Event>>,
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
            clients: HashMap::new(),
            nicks: HashMap::new(),
            channels: HashMap::new(),
            tx: None
        })
    }
    
    /// Starts the main loop and listens on the specified host and port.
    pub fn serve_forever(mut self) -> IoResult<Server> {
        use self::Event::{Connected, InboundMessage};
        // todo change this to a more general event dispatching loop
        let (tx, rx) = try!(self.listen());
        self.tx = Some(tx);
        for event in rx.iter() {
            match event {
                InboundMessage(id, msg) => {
                    if let Some(client) = self.clients.get(&id).map(|c| c.clone()) {
                        message_handler::invoke(msg, &mut self, client)
                    }
                    
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
    pub fn send_response(&self, client: &Client, code: ResponseCode, payload: &[&str]) {
        client.send_response(code, payload);
    }

    /// Sends a response to the client
    pub fn send_msg(&self, client: &Client, cmd: Command, payload: &[&[u8]]) {
        client.send_msg(cmd, payload, MessageOrigin::Server);
    }

    pub fn register(&self, client: &Client) {
        self.send_welcome_msg(client)
    }
    
    /// Sends a welcome message to a newly registered client
    fn send_welcome_msg(&self, client: &Client) {
        let info = client.info();
        self.send_response(client, ResponseCode::RPL_WELCOME, &[&*format!(
            "Welcome to the Internet Relay Network {nick}!{user}@{host}", 
            nick=info.nick(),
            user=info.user(),
            host=info.host()
        )])
    }

    /// Getter for channels
    pub fn channels(&self) ->  &HashMap<String, channel::Proxy> {
        &self.channels
    }

    /// Getter for mut channels
    pub fn channels_mut(&mut self) ->  &mut HashMap<String, channel::Proxy> {
        &mut self.channels
    }

    /// Getter for nicks
    pub fn nicks(&self) ->  &HashMap<String, ClientId> {
        &self.nicks
    }

    /// Getter for nicks
    pub fn nicks_mut(&mut self) ->  &mut HashMap<String, ClientId> {
        &mut self.nicks
    }

    /// Gets a client
    pub fn client_with_name(&self, name: &str) -> Option<&Client> {
        match self.nicks.get(name) {
            Some(id) => self.clients.get(id),
            None => None
        }
    }

    /// Getter for tx for sending to main event loop
    /// Panics if the main loop is not started
    pub fn tx(&mut self) ->  &Sender<Event> {
        self.tx.as_ref().unwrap()
    }
}

#[cfg(test)]
pub fn get_test_server() -> Server {
    Server {
        host: "testserver.example.com".to_string(),
        ip: "127.0.0.1".to_string(),
        port: 0,
        clients: HashMap::new(),
        nicks: HashMap::new(),
        channels: HashMap::new(),
        tx: None,
    }
}