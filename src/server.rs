//! Server model

use std::io;
use std::net;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::spawn;
use std::collections::HashMap;

use mio::{EventLoopSender, EventLoop, Handler, Token};
use mio;

use protocol::{Command, ResponseCode, Message};
use client::{ClientId, Client, MessageOrigin};
use client_io;
use message_handler;
use channel;
use services::{Service, NickServ, Action};

pub struct Server {
    host: String,
    ip: String,
    port: u16,
    clients: HashMap<ClientId, Client>,
    nicks: HashMap<String, ClientId>,
    channels: HashMap<String, channel::Proxy>,
    listener: Option<mio::tcp::TcpListener>,
    server_tx: Option<EventLoopSender<Event>>,
    client_tx: Option<EventLoopSender<client_io::Event>>,
    services: HashMap<String, Rc<RefCell<Box<Service>>>>,
}

pub enum Event {
    Connected(Client),
    Disconnected(Client),
    InboundMessage(ClientId, Message)
}

/// Irc server
impl Server {
    /// Creates a new IRC server instance.
    pub fn new(host: &str) -> io::Result<Server> {
        let addresses = try!(net::lookup_host(host));
        // Listen only on ipv4 for now…
        let ip = match addresses.filter_map(|v| v.ok()).filter(
            |&v| match v { net::SocketAddr::V4(_) => true, _ => false }
        ).nth(0) {
            Some(ip) => ip,
            None => return Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot get host IP address.",
                None
            ))
        };
        Ok(Server {
            host: host.to_string(),
            ip: format!("{}", ip),
            port: 6667,
            clients: HashMap::new(),
            nicks: HashMap::new(),
            channels: HashMap::new(),
            listener: None,
            server_tx: None,
            client_tx: None,
            services: HashMap::new(),
        })
    }

    pub fn run_mio(&mut self) -> io::Result<()>  {
        let mut server_loop = try!(EventLoop::new());
        let mut client_loop = try!(EventLoop::new());
        self.server_tx = Some(server_loop.channel());
        self.client_tx = Some(client_loop.channel());
		// TODO listen to all IP addresses (move lookup_host to here)
		self.listener = Some(try!(mio::tcp::TcpListener::bind(&*format!("{}:{}", self.ip, self.port))));
		info!("started listening on {}:{} ({})", self.ip, self.port, self.host);
        try!(server_loop.register(self.listener.as_ref().unwrap(), Token(self.port as usize)));
        let host = Arc::new(self.host.clone());
        let tx = server_loop.channel();
        spawn(move || {
            use client_io::Worker;
            let _ = client_loop.run(&mut Worker::new(tx, host)).unwrap();
        });
        server_loop.run(self)
    }

    /// Sends a response to the client
    pub fn send_response(&self, client: &Client, code: ResponseCode, payload: &[&str]) {
        client.send_response(code, payload);
    }

    /// Sends a response to the client
    pub fn send_msg(&self, client: &Client, cmd: Command, payload: &[&str]) {
        client.send_msg(cmd, payload, MessageOrigin::Server);
    }

    /// Sends a response to the client
    pub fn send_raw_msg(&self, client: &Client, cmd: Command, payload: &[&[u8]]) {
        client.send_raw_msg(cmd, payload, MessageOrigin::Server);
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

    /// Getter for services
    pub fn with_service<'a, F>(&'a mut self, name: String, mut f: F) -> Action<'a>
    where F: FnMut(&mut Service, &'a mut Server) -> Action<'a> {
        if let Some(service) = self.services.get(&name).map(|v| v.clone()) {
            f(&mut **service.borrow_mut(), self)
        } else {
            Action::Continue(self)
        }
    }

    /// Getter for tx for sending to main event loop
    /// Panics if the main loop is not started
    pub fn tx(&mut self) ->  &EventLoopSender<Event> {
        self.server_tx.as_ref().unwrap()
    }
}

impl Handler for Server {
    type Timeout = ();
    type Message = Event;

    fn notify(&mut self, _: &mut EventLoop<Server>, msg: Event) {
        use self::Event::*;
        match msg {
            InboundMessage(id, msg) => {
                if let Some(client) = self.clients.get(&id).map(|c| c.clone()) {
                    message_handler::invoke(msg, self, client)
                }
                
            }
            Connected(client) => {
                let id = client.id();
                self.clients.insert(id, client);
            }
            Disconnected(client) => {
                self.clients.remove(&client.id());
                self.nicks.remove(&*client.nick());
            }
        }
    }
    fn readable(&mut self, _: &mut EventLoop<Server>, _: Token, _: mio::ReadHint) {
        if let Ok((stream, _)) = self.listener.as_ref().unwrap().accept() {
            let _ = self.client_tx.as_ref().unwrap().send(client_io::Event::NewConnection(stream));
        } 
    }
}

#[cfg(test)]
pub fn get_test_server() -> Server {
    Server {
        host: "localhost".to_string(),
        ip: "127.0.0.1".to_string(),
        port: 6667,
        clients: HashMap::new(),
        nicks: HashMap::new(),
        channels: HashMap::new(),
        listener: None,
        server_tx: None,
        client_tx: None,
        nick_serv: None
    }
}