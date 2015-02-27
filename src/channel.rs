use std::sync::{Arc, Weak, RwLock};
use std::sync::mpsc::{Sender};

use protocol::Message;
use client::Client;

/// Member of a channel
struct Member {
    client: Client
}

/// Possible events that can be sent to a channel
enum Event {
    Dispatch(Message)
}

/// An IRC channel.
///
/// The IRC channel object manages it’s own members.
/// This includes authentification, per channel bans etc.
pub struct Channel {
    members: Vec<Member>
}