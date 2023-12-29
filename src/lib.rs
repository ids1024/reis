#![forbid(unsafe_code)]

// TODO error type?
// TODO split up
// Implement handshake

use std::{
    collections::{self, VecDeque},
    env, fmt, iter,
    os::unix::io::OwnedFd,
    path::PathBuf,
    string::FromUtf8Error,
};

mod arg;
use arg::{Arg, OwnedArg};
mod backend;
use backend::Backend;
pub use backend::{ConnectionReadResult, PendingRequestResult}; // XXX types? names?
pub mod ei;
mod eiproto_ei;
mod eiproto_eis;
mod eiproto_enum;
pub mod eis;
pub mod event; // XXX reorganize?
pub mod handshake; // XXX ^
mod eis_event;
mod object;
pub use object::Object;
mod util;

#[cfg(feature = "tokio")]
pub mod tokio;

mod private {
    pub trait Sealed {}
}

// TODO versioning?

// XXX
// Want to fallback to higher number if exists, on server?
// create on server, not client.
pub fn default_socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}

#[derive(Debug)]
struct Header {
    object_id: u64,
    length: u32,
    opcode: u32,
}

impl Header {
    fn parse(bytes: [u8; 16]) -> Self {
        Self {
            object_id: u64::from_ne_bytes(bytes[0..8].try_into().unwrap()),
            length: u32::from_ne_bytes(bytes[8..12].try_into().unwrap()),
            opcode: u32::from_ne_bytes(bytes[12..16].try_into().unwrap()),
        }
    }

    fn as_bytes(&self) -> impl Iterator<Item = u8> {
        self.object_id
            .to_ne_bytes()
            .into_iter()
            .chain(self.length.to_ne_bytes())
            .chain(self.opcode.to_ne_bytes())
    }
}

#[doc(hidden)]
pub trait Interface: private::Sealed {
    const NAME: &'static str;
    const VERSION: u32;
    const CLIENT_SIDE: bool;
    type Incoming;

    fn new_unchecked(object: Object) -> Self;

    fn as_arg(&self) -> Arg<'_>;
}

trait MessageEnum {
    fn args(&self) -> Vec<crate::Arg<'_>>;
}

struct ByteStream<'a> {
    backend: &'a Backend,
    bytes: std::collections::vec_deque::Drain<'a, u8>,
    fds: &'a mut VecDeque<OwnedFd>,
}

impl<'a> ByteStream<'a> {
    fn backend(&self) -> &Backend {
        self.backend
    }

    // TODO: Using impl Iterator ran into lifetime issues
    fn read_n<'b>(
        &'b mut self,
        n: usize,
    ) -> Result<iter::Take<&'b mut collections::vec_deque::Drain<'a, u8>>, ParseError> {
        if self.bytes.len() >= n {
            Ok(self.bytes.by_ref().take(n))
        } else {
            Err(ParseError::EndOfMessage)
        }
    }

    fn read<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        Ok(util::array_from_iterator_unchecked(self.read_n(N)?))
    }

    fn read_fd(&mut self) -> Result<OwnedFd, ParseError> {
        self.fds.pop_front().ok_or(ParseError::NoFd)
    }

    fn read_arg<T: OwnedArg>(&mut self) -> Result<T, ParseError> {
        T::parse(self)
    }
}

#[derive(Debug)]
pub enum ParseError {
    EndOfMessage,
    Utf8(FromUtf8Error),
    InvalidId(u64),
    NoFd,
    InvalidOpcode(&'static str, u32),
    InvalidVariant(&'static str, u32),
    InvalidInterface(String),
    HeaderLength(u32),
    MessageLength(u32, u32),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::EndOfMessage => write!(f, "found end of message while parsing argument"),
            Self::Utf8(e) => write!(f, "invalid UTF-8 string in message: {}", e),
            Self::InvalidId(id) => write!(f, "new object id '{}' invalid", id),
            Self::NoFd => write!(f, "expected fd"),
            Self::InvalidOpcode(intr, op) => {
                write!(f, "opcode '{}' invallid for interface '{}'", op, intr)
            }
            Self::InvalidVariant(enum_, var) => {
                write!(f, "variant '{}' invallid for enum '{}'", var, enum_)
            }
            Self::InvalidInterface(intr) => write!(f, "unknown interface '{}'", intr),
            Self::HeaderLength(len) => write!(f, "header length {} < 16", len),
            Self::MessageLength(a, b) => {
                write!(f, "message length didn't match header ({} != {})", a, b)
            }
        }
    }
}

impl From<FromUtf8Error> for ParseError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl std::error::Error for ParseError {}
