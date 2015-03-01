//! Channel model

use std::collections::{HashMap, HashSet};
use std::collections::hash_map;
use std::sync::mpsc::{Sender, channel};
use std::sync::Arc;
use std::thread::spawn;
use std::thunk::Invoke;


use server;
use protocol::{ResponseCode};
use user::HostMask;
use client::{ClientId, Client};
use client;

pub use self::Event::*;
pub use super::{Member, Flags, ChannelMode, modes_do};


/// Forwards the message to a channel
pub struct Proxy {
    name: String,
    tx: Sender<Event>,
    server_tx: Sender<server::Event>
}

impl Proxy {
    fn new(name: String,
           tx: Sender<Event>, 
           server_tx: Sender<server::Event>) -> Proxy {
        Proxy {
            name: name,
            tx: tx,
            server_tx: server_tx
        }
    }

    /// Sends an event to the channel
    pub fn send(&self, event: Event) {
        match self.tx.send(event) {
            Ok(_) => {},
            Err(_) => {
                //let _ = self.server_tx.send_opt(server::ChannelLost(self.name.clone()));
            }
        }
    }
}


/// Enumeration of events a channel can receive
// TODO replace with FnOnce and remove 'static
pub enum Event {
    Handle(Box<for<'r> Invoke<(&'r Channel)> + Send>),
    HandleMut(Box<for<'r> Invoke<(&'r mut Channel)> + Send>),
}
/*
/// Enumeration of events a channel can receive
// TODO replace with FnOnce and remove 'static
pub enum Event {
    Handle(Box<FnOnce(&Channel) + Send>),
    HandleMut(Box<FnOnce(&mut Channel) + Send>),
}
*/

/// An IRC channel.
///
/// The IRC channel object manages it’s own members.
/// This includes authentification, per channel bans etc.
pub struct Channel {
    name: String,
    topic: Vec<u8>,
    password: Option<Vec<u8>>,
    flags: Flags,
    limit: Option<usize>,
    members: HashMap<String, Member>,
    nicknames: HashMap<ClientId, String>,
    ban_masks: HashSet<HostMask>,
    except_masks: HashSet<HostMask>,
    invite_masks: HashSet<HostMask>,
}

fn to_invoke<F>(func: F) -> F
where F : FnOnce(&Channel) + Send {
    func
}

impl Channel {
    pub fn new(name: String) -> Channel {
        Channel {
            name: name,
            topic: b"".to_vec(),
            password: None,
            flags: HashSet::new(),
            limit: None,
            members: HashMap::new(),
            nicknames: HashMap::new(),
            ban_masks: HashSet::new(),
            except_masks: HashSet::new(),
            invite_masks: HashSet::new(),
        }
    }
    
    /// Starts listening for events in a separate thread
    pub fn listen(self, server_tx: Sender<server::Event>) -> Proxy {
        let (tx, rx) = channel();
        let name = self.name.clone();
        spawn(move || {
            let mut this = self;
            for event in rx.iter() {
                this.dispatch(event)
            }
        });
        Proxy::new(name, tx, server_tx)
    }

    /// Message dispatcher
    fn dispatch(&mut self, event: Event) {
        use std::mem; // workaround until FnOnce is object safe
        match event {
            Handle(handler) => handler.invoke(self),
            HandleMut(handler) => handler.invoke(self),
            //Message(command, client_id, message) => {
            //    match command {
            //        PRIVMSG => self.handle_privmsg(client_id, message),
            //    }
            //}
        }
    }
    
    /// Getter for channel name
    pub fn name(&self) -> &str {
        self.name.as_slice()
    }
    
    /// Getter for topic
    pub fn topic(&self) -> &[u8] {
        self.topic.as_slice()
    }
    
    /// Setter for topic
    pub fn set_topic(&mut self, topic: Vec<u8>) {
        self.topic = topic
    }
    
    /// Getter for the user limit
    pub fn limit(&self) -> Option<usize> {
        self.limit
    }
    /// Setter for the user limit
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit
    }
    
    /// Getter for the channel password
    pub fn password(&self) -> &Option<Vec<u8>> {
        &self.password
    }
    /// Setter for the channel password
    pub fn set_password(&mut self, password: Option<Vec<u8>>) {
        self.password = password
    }
    
    /// Returns the member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
    
    /// Returns a view into the channel members
    pub fn members<'a>(&'a self) -> hash_map::Values<'a, String, Member> {
        self.members.values()
    }
    
    /// Adds a flag to the channel
    pub fn add_flag(&mut self, flag: ChannelMode) -> bool {
        self.flags.insert(flag)
    }
    
    /// Removes a flag from the channel
    pub fn remove_flag(&mut self, flag: ChannelMode) -> bool {
        self.flags.remove(&flag)
    }
    
    /// Checks if the channel has flag `flag`
    pub fn has_flag(&self, flag: ChannelMode) -> bool {
        self.flags.contains(&flag)
    }
    
    /// Channel flags as a string
    pub fn flags(&self) -> String {
        self.flags.iter().map( |c| *c as u8 as char).collect() 
    }
    
    /// Adds a ban mask to the channel
    pub fn add_ban_mask(&mut self, mask: HostMask) -> bool {
        self.ban_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_ban_mask(&mut self, mask: HostMask) -> bool {
        self.ban_masks.remove(&mask)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_except_mask(&mut self, mask: HostMask) -> bool {
        self.except_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_except_mask(&mut self, mask: HostMask) -> bool {
        self.except_masks.remove(&mask)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_invite_mask(&mut self, mask: HostMask) -> bool {
        self.invite_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_invite_mask(&mut self, mask: HostMask) -> bool {
        self.invite_masks.remove(&mask)
    }
    
    /// Getter for the ban masks
    pub fn ban_masks(&self) -> &HashSet<HostMask> {
        &self.ban_masks
    }
    
    /// Getter for the ban masks
    pub fn except_masks(&self) -> &HashSet<HostMask> {
        &self.except_masks
    }
    
    /// Getter for the ban masks
    pub fn invite_masks(&self) -> &HashSet<HostMask> {
        &self.invite_masks
    }
    
    /// Adds a member to the channel
    pub fn add_member(&mut self, member: Member) -> bool {
        if self.member_with_id(member.id()).is_some() {
            false // member already in channel
        } else {
            self.nicknames.insert(member.id(), member.nick().to_string());
            self.members.insert(member.nick().to_string(), member);
            true
        }
    }
    
    /// Adds a member to the channel
    pub fn remove_member(&mut self, id: &ClientId) -> bool {
        let nick = { match self.nicknames.get(id) {
                Some(nick) => nick.clone(),
                None => return false
        }};
        self.nicknames.remove(id);
        self.members.remove(&nick);
        true
    }
    
    pub fn send_response(&self, client: &Client, command: ResponseCode, 
                         params: &[&str]) {
        client.send_response(
            command, 
            params,
        )
    }
    
    pub fn member_with_id(&self, client_id: ClientId) -> Option<&Member> {
        let nick = self.nicknames.get(&client_id).clone();
        match nick {
            Some(nick) => self.members.get(nick),
            None => None
        }
    }
    
    pub fn mut_member_with_id(&mut self, client_id: ClientId) -> Option<&mut Member> {
        let nick = self.nicknames.get(&client_id).clone();
        match nick {
            Some(nick) => self.members.get_mut(nick),
            None => None
        }
    }
    
    pub fn member_with_nick(&self, nick: &String) -> Option<&Member> {
        self.members.get(nick)
    }
    
    pub fn mut_member_with_nick(&mut self, nick: &String) -> Option<&mut Member> {
        self.members.get_mut(nick)
    }
    
    /// Broadcasts a message to all members
    #[inline]
    pub fn broadcast_raw(&self, msg: Arc<Vec<u8>>) {
        for member in self.members() {
            member.send(client::Event::SharedMessage(msg.clone()))
        }
    }

    pub fn list_sender<'a>(&'a self, receiver: &'a Client, list_code: ResponseCode,
    end_code: ResponseCode) -> ListSender {
        ListSender {
            receiver: receiver,
            list_code: list_code,
            end_code: end_code,
            name: self.name(),
        }
    }
}

/// Helper struct to send list replies
pub struct ListSender<'a> {
    receiver: &'a Client,
    list_code: ResponseCode,
    end_code: ResponseCode,
    name: &'a str,
}
impl<'a> ListSender<'a> {
    /// Sends a list item to the sender
    ///
    /// The sender prepends the list item with the channel name.
    pub fn feed_line(&self, line: &[&str]) {
        self.receiver.send_response(
            self.list_code, 
            (vec![self.name] + line.as_slice()).as_slice()
        )
    }
    /// Tells the sender that there are no more items in the list
    ///
    /// Note: this happens automatically when the sender is dropped.
    pub fn end_of_list(self) {
        drop(self)
    }
}
#[unsafe_destructor]
impl<'a> Drop for ListSender<'a> {
    fn drop(&mut self) {
        self.receiver.send_response(self.end_code, &[self.name])
    }
}