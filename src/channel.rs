use protocol::Message;
use client::Client;

/// Member of a channel
struct Member {
    client: Client
}

/// Possible events that can be sent to a channel
pub enum Event {
    Handle(Box<FnOnce(&Channel)>),
    HandleMut(Box<FnOnce(&mut Channel)>)
}

/// An IRC channel.
///
/// The IRC channel object manages it’s own members.
/// This includes authentification, per channel bans etc.
pub struct Channel {
    members: Vec<Member>
}