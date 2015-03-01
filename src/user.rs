//! Module containing everything related to users
use std::mem;
use client::ClientId;

#[derive(Debug, PartialEq, Copy)]
pub enum Status {
    /// User is not connected
	Disconnected,
    /// User did just connect
	Connected,
    /// User successfully registered a nickname
	NickRegistered,
    /// User sent a USER command
    NameRegistered,
    /// User initiated capability negotiations
    Negotiating(&'static Status),
    /// User is fully registered
	Registered
}
pub const STATUS_NEG_NAMEREG: Status = Status::Negotiating(&Status::NameRegistered);
pub const STATUS_NEG_NICKREG: Status = Status::Negotiating(&Status::NickRegistered);
pub const STATUS_NEG_CONNECT: Status = Status::Negotiating(&Status::Connected);
pub const STATUS_NEG_REG: Status = Status::Negotiating(&Status::Registered);

#[derive(Debug)]
pub struct User {
    nick: String,
    user: String,
    realname: String,
    host: String,
    status: Status,
    hostmask: HostMask
}

impl User {
    /// Creates the user info struct.
    pub fn new(host: String) -> User {
        let mask = HostMask::from_parts("*", "*", &*host);
        User {
            nick: "*".to_string(),
            user: "".to_string(),
            realname: "John Doe".to_string(),
            host: host,
            hostmask: mask,
            status: Status::Connected
        }
    }
    
    /// Getter for the nick name
    pub fn nick(&self) -> &str {
        &*self.nick
    }
    /// Setter for the nick name
    pub fn set_nick(&mut self, nick: String) -> String {
    	let nick = mem::replace(&mut self.nick, nick);
        self.update_mask();
        nick
    }
    /// Getter for the nick name
    pub fn user(&self) -> &str {
        &*self.user
    }
    /// Getter for the user name
    pub fn set_user(&mut self, name: String) {
        self.user = name;
        self.update_mask()
    }
    /// Getter for the nick name
    pub fn realname(&self) -> &str {
        &*self.realname
    }
    /// Getter for the real name
    pub fn set_realname(&mut self, name: String) {
        self.realname = name;
        self.update_mask()
    }
    /// Getter for the server name
    pub fn host(&self) -> &str {
        &*self.host
    }
    /// Getter for the registration status/method
    pub fn status(&self) -> Status {
        self.status
    }
    /// Getter for the registration status/method
    pub fn set_status(&mut self, status: Status) {
        self.status = status
    }
    
    /// Updates the real hostmask
    fn update_mask(&mut self) {
        self.hostmask = HostMask::from_parts(
            &*self.nick,
            &*self.user,
            &*self.host
        )
    }

    /// Getter for the public host mask.
    ///
    /// This is the host mask that is send out to other users.
    pub fn public_hostmask(&self) -> &HostMask {
        &self.hostmask
    }
    /// Getter for the real host mask
    pub fn real_hostmask(&self) -> &HostMask {
        &self.hostmask
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
/// A host mask in the form "*!*@*.*"
pub struct HostMask {
    mask: String
}

impl HostMask {
    pub fn new(mask: String) -> HostMask {
        HostMask {
            mask: mask
        }
    }
    pub fn from_parts(nick: &str, name: &str, host: &str) -> HostMask {
        HostMask {
            mask: format!("{}!{}@{}", nick, name, host)
        }
    }
    /// checks if the host mask matches another mask
    ///
    /// "*!*@*.com" would match "a!b@example.com"
    pub fn matches(&self, mask: &str) -> bool {
        let mut mask_chars = mask.chars().peekable();
        let mut chars = self.mask.as_slice().chars().peekable();
        loop {
            match chars.next() {
                Some('*') => match chars.peek() {
                    // Consume all chars until next match is found
                    Some(next) => while match mask_chars.peek() {
                        Some(mask_cha) => mask_cha != next,
                        None => false } { let _ = mask_chars.next(); },
                    // * at end of the string matches the whole rest
                    None => return true
                },
                Some(cha) => match mask_chars.next() {
                    None => return false,
                    Some(mask_cha) => if cha != mask_cha { return false }
                },
                None => break
            }
        }
        !mask_chars.next().is_some()
    }
    
    /// Returns the hostname
    pub fn host(&self) -> Option<&str> {
        self.mask.as_slice().split('@').last()
    }
    /// Returns the username
    pub fn user(&self) -> Option<&str> {
        self.mask.as_slice().split('@').nth(0).and_then(|v| 
            v.split('!').last()
        )
    }
    /// Returns the nickname
    pub fn nick(&self) -> Option<&str> {
        self.mask.as_slice().split('!').nth(0)
    }
    
    /// Returns a view into the mask
    pub fn as_str(&self) -> &str {
        return self.mask.as_slice()
    }
}


#[cfg(test)]
mod tests {
    use super::HostMask;
    
    #[test]
    /// Test the hostname masks
    fn mask_matching() {
        assert!(HostMask::new("*!*@*.com".to_string()).matches("a!b@example.com"));
        assert!(!HostMask::new("*!*@*.com".to_string()).matches("*!*@*.edu"));
        assert!(!HostMask::new("*!*@*.com".to_string()).matches("*!*@*.cop"));
        assert!(!HostMask::new("*!*@*.com".to_string()).matches("*!*@*.coma"));
        assert!(HostMask::new("*!*@example.com".to_string()).matches("a!b@example.com"));
        assert!(HostMask::new("foo!*@*.com".to_string()).matches("foo!bar@example.com"));
        assert!(!HostMask::new("foo!*@*.com".to_string()).matches("baz!bar@example.com"));
        assert!(HostMask::new("*!bar@*.com".to_string()).matches("foo!bar@example.com"));
        assert!(!HostMask::new("*!bar@*.com".to_string()).matches("foo!baz@example.com"));
    }
    
}
