//mod member;
use std::collections::HashSet;
use std::num::FromPrimitive;

use protocol::{Message, Params};
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

/// Enumeration of possible channel modes
/// as of http://tools.ietf.org/html/rfc2811#section-4
#[derive(FromPrimitive, Debug, Hash, PartialEq, Eq, Copy)]
pub enum ChannelMode {
    /// give "channel creator" status
    ChannelCreator = b'O' as isize,
    /// give/take channel operator privilege
    OperatorPrivilege = b'o' as isize,
    /// give/take the voice privilege
    VoicePrivilege = b'v' as isize,
    /// toggle the anonymous channel flag
    AnonChannel = b'a' as isize,
    /// toggle the invite-only channel flag
    InviteOnly = b'i' as isize,
    /// toggle the moderated channel
    Moderated = b'm' as isize,
    /// toggle the no messages to channel from clients on the
    /// outside
    MemberOnly = b'n' as isize,
    /// toggle the quiet channel flag
    Quiet = b'q' as isize,
    /// toggle the private channel flag
    Private = b'p' as isize,
    /// toggle the secret channel flag
    Secret = b's' as isize,
    /// toggle the server reop channel flag
    ReOpFlag = b'r' as isize,
    /// toggle the topic settable by channel operator only flag
    TopicProtect = b't' as isize,
    /// set/remove the channel key (password)
    ChannelKey = b'k' as isize,
    /// set/remove the user limit to channel
    UserLimit = b'l' as isize,
    /// set/remove ban mask to keep users out
    BanMask = b'b' as isize,
    /// set/remove an exception mask to override a ban mask
    ExceptionMask = b'e' as isize,
    /// set/remove an invitation mask to automatically override
    /// the invite-only flag
    InvitationMask = b'I' as isize
}

// Actions which determine what to do with a mode
#[derive(PartialEq, Eq, Debug, Copy)]
pub enum Action {
    // Add a flag
    Add,
    // Remove a flag
    Remove,
    // Show the flag
    Show
}

impl ChannelMode {
    fn has_parameter(&self) -> bool {
    	use self::ChannelMode::*;
        match *self {
            ChannelKey | UserLimit | BanMask
            | ExceptionMask | InvitationMask
            | OperatorPrivilege | VoicePrivilege => true,
            _ => false
        }
    }
}

/// Parses the channel modes
///
/// According to [RFC 2812] (http://tools.ietf.org/html/rfc2812#section-3.2.3) the
/// syntax for setting modes is:
/// ```
///    Command: MODE
/// Parameters: <channel> *( ( "-" / "+" ) *<modes> *<modeparams> )
/// ```
///
/// Additionally an example is given
///
/// ```
/// MODE &oulu +b *!*@*.edu +e *!*@*.bu.edu
///                                 ; Command to prevent any user from a
///                                 hostname matching *.edu from joining,
///                                 except if matching *.bu.edu
/// 
/// MODE #bu +be *!*@*.edu *!*@*.bu.edu
///                                 ; Comment to prevent any user from a
///                                 hostname matching *.edu from joining,
///                                 except if matching *.bu.edu
/// ```
/// 
/// 
pub fn modes_do<Block>(mut params: Params, mut block: Block)
where Block: FnMut(Action, ChannelMode, Option<&[u8]>) {
	use self::Action::*;
	while let Some(current) = params.next() {
        // Bug: no +/- asking for modes
        let (action, offset) = match current[0] {
            b'+' => (Add, 1),
            b'-' => (Remove, 1),
            _ => (Show, 0)
            
        };
        for mode in current[offset..].iter().filter_map( |&v| {
            let m: Option<ChannelMode> = FromPrimitive::from_u8(v); m
        }) {
            let param = if mode.has_parameter() && action != Show {
                let param = params.next();
                param
            } else {
                None
            };
            block(action, mode, param);
        }
	}

}

/// List of channel modes / member flags
pub type Flags = HashSet<ChannelMode>;


/// An IRC channel.
///
/// The IRC channel object manages it’s own members.
/// This includes authentification, per channel bans etc.
pub struct Channel {
    members: Vec<Member>
}

#[cfg(test)]
mod tests {
	use super::{modes_do};
	use super::ChannelMode::*;
	use super::Action::*;
	use protocol::Message;
	/// Tests the mode parser
    
    
    
	#[test]
	fn test_mode_parser() {
        let msgs = [
            b"MODE &oulu +b *!*@*.edu +e *!*@*.bu.edu",
            b"MODE #bu +be *!*@*.edu *!*@*.bu.edu",
            b"MODE #bu b",
            b"MODE #test -oo Guest",
            // TODO fix this test
            //b"MODE #bu /i", // Invalid mode should be skipped
            b"MODE #bu +g", // Invalid mode should be skipped
        ];
        let modes = [
            vec![(Add, BanMask, Some(b"*!*@*.edu")),
            (Add, ExceptionMask, Some(b"*!*@*.bu.edu"))],
            vec![(Add, BanMask, Some(b"*!*@*.edu")),
            (Add, ExceptionMask, Some(b"*!*@*.bu.edu"))],
            vec![(Show, BanMask, None)],
            vec![(Remove, OperatorPrivilege, Some(b"Guest")),
            (Remove, OperatorPrivilege, None)],
            //Vec::new(),
            Vec::new(),
        ];
        for (msg, modes) in msgs.iter().zip(modes.iter()) {
            let m = Message::new(msg.to_vec()).unwrap();
            let mut mode_iter = modes.iter();
            let mut params = m.params();
            let _ = params.next();
            modes_do(params, |set, mode, parameter| {
                let &(set_, mode_, parameter_) = mode_iter.next().unwrap();
                assert_eq!(set_, set);
                assert_eq!(mode_, mode);
                assert_eq!(parameter_, parameter);
            })
        }
	}
}