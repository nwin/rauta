use std::ops::Range;
use std::ascii::AsciiExt;
use std::fmt;
use std::ops;

/// An IRC message
#[derive(Clone)]
pub struct Message {
    message: Vec<u8>,
    //tags: Vec<Range<usize>>
    prefix: Option<Range<usize>>,
    command: Range<usize>,
    params: Vec<Range<usize>>
}

/// Searches a slice for the first occurence of needle
fn position<T: PartialEq>(this: &[T], needle: &[T]) -> Option<usize> {
    this.windows(needle.len()).position(|v| v == needle)
}

/// A parser for irc messages.
///
/// The parser is aware of IRCv3.2 message tags but does not evaluate them
/// TODO: get rid of the allocations
impl Message {
    pub fn new(message: Vec<u8>) -> Result<Message, &'static str> {
        let mut this = Message {
            message: message,
            // TODO see if we could to better than guessing
            // guess = 0
            //tags: Vec::new(),
            prefix: None,
            command: 0..0,
            // TODO see if we could to better than guessing
            // guess = 5 tags per message
            params: Vec::with_capacity(5)
        };
        try!(this.init());
        Ok(this)
    }
    
    /// Parses the message.
    fn init(&mut self) -> Result<(), &'static str> {
        let mut message = &*self.message;
        // Tag section starts with `b'@'` and ends with `b' '`
        let prefix_start = if message.starts_with(&[b'@']) {
            let prefix_start = match message.position_elem(&b' ') { 
                Some(v) => v + 1, 
                None => return Err("Message does not contain a command.") 
            };
            // Just ignore tags for now
            message = &message[prefix_start..];
            prefix_start
        } else {
            0
        };
        // Prefix starts with `b':'` and ends with `b' '`
        self.prefix = if message.starts_with(&[b':']) {
            let prefix_end = match message.position_elem(&b' ') { 
                Some(v) => v, 
                None => return Err("Message does not contain a command.") 
            };
            message = &message[prefix_end + 1..];
            Some(prefix_start + 1..prefix_start + prefix_end)
        } else {
            None
        };
        let cmd_start = self.prefix.as_ref().map(|v| v.end + 1).unwrap_or(0);
        let trailing = match position(message, b" :") {
            Some(trailing_pos) => {
                message = &message[..trailing_pos];
                Some(cmd_start + trailing_pos + 2..self.message.len())
            },
            None => None
        };
        // Middle part as of RFC 1459
        let mut middle = message.split(|&x| x == b' ');
        self.command = match middle.next() {
            Some(cmd) => {
                // Make sure that the command is valid ASCII
                // since no non-ASCII command has been specified so far.
                // Unsafe code in `Message::command` relies on this.
                if cmd.is_ascii() {
                    cmd_start..cmd_start + cmd.len()
                } else {
                    return Err("Command contains non-ASCII characters.")
                }
            },
            None => return Err("Message does not contain a command.") 
        };
        let mut start = self.command.end + 1;
        self.params.clear();
        for param in middle {
            let slice = start..start + param.len();
            start = slice.end + 1;
            self.params.push(slice)
        }
        if let Some(suffix) = trailing {
            self.params.push(suffix)
        }
        Ok(())
    }
    
    /// Returns the message prefix
    /// It might contain non-utf8 chars and thus only bytes are returned.
    pub fn prefix(&self) -> Option<&[u8]> {
        self.prefix.as_ref().map(|range| &self.message[*range])
    }
    
    /// Returns the command part of the message.
    pub fn command(&self) -> &str {
        use std::mem;
        // This is safe because the parser already checked that the
        // command only contains valid ASCII chars
        unsafe { mem::transmute(&self.message[self.command]) }
    }
    
    /// Returns the parameters of the command.
    ///
    /// *Note* since the IRC protocol does not define any encoding
    /// raw bytes are returned.
    pub fn params(&self) -> Params {
        Params {
            msg: self,
            i: 0 - 1
        }
    }

    /// Consumes the message and returns the underlying vec
    pub fn into_vec(self) -> Vec<u8> {
        self.message
    }
}

impl ops::Deref for Message {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.message
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt, 
            "Message {{ message: {:?}, prefix: {:?}, command: {:?}, params: {:?} }}",
            String::from_utf8_lossy(&self.message),
            self.prefix,
            self.command,
            self.params
        )
    }
}

/// Iterator over the parameters of a message
pub struct Params<'a> {
    msg: &'a Message,
    i: usize
}

impl<'a> Iterator for Params<'a> {
    type Item = &'a [u8];
    
    fn next(&mut self) -> Option<&'a [u8]> {
        self.i += 1;
        self.msg.params.get(self.i).map( |range|
            &self.msg.message[*range]
        )
    }
}


#[cfg(test)]
mod tests {
    extern crate test;
	use super::Message;
	/// Test the nickname validation function
	#[test]
	fn message_parser() {
        let m = Message::new("@tag :prefix JOIN #channel".as_bytes().to_vec()).unwrap();
        assert_eq!(m.prefix().unwrap(), b"prefix");
        assert_eq!(m.command(), "JOIN");
        assert_eq!(m.params().nth(0).unwrap(), b"#channel");
	}
    #[bench]
    fn bench_parser(b: &mut test::Bencher) {
        let message = b":prefix JOIN #channel".to_vec();
        b.iter(|| {
            test::black_box(Message::new(message.clone()).unwrap());
        });
        b.bytes = message.len() as u64
    }
}