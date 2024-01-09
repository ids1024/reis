/// EI wire protcol.
///
/// This is the lowest level component of reis. It provides serialization
/// and deserialization of the protocol, and uses Rustix to handle socket IO.
use std::{
    collections::{self, VecDeque},
    fmt, iter,
    os::unix::io::OwnedFd,
    string::FromUtf8Error,
};

use crate::{Backend, Object};

mod arg;
pub(crate) use arg::{Arg, OwnedArg};

#[derive(Debug)]
pub(crate) struct Header {
    pub object_id: u64,
    pub length: u32,
    pub opcode: u32,
}

impl Header {
    pub fn parse(bytes: [u8; 16]) -> Self {
        Self {
            object_id: u64::from_ne_bytes(bytes[0..8].try_into().unwrap()),
            length: u32::from_ne_bytes(bytes[8..12].try_into().unwrap()),
            opcode: u32::from_ne_bytes(bytes[12..16].try_into().unwrap()),
        }
    }

    pub fn as_bytes(&self) -> impl Iterator<Item = u8> {
        self.object_id
            .to_ne_bytes()
            .into_iter()
            .chain(self.length.to_ne_bytes())
            .chain(self.opcode.to_ne_bytes())
    }
}

#[doc(hidden)]
pub trait Interface: crate::private::Sealed {
    const NAME: &'static str;
    const VERSION: u32;
    const CLIENT_SIDE: bool;
    type Incoming;

    fn new_unchecked(object: Object) -> Self;

    fn as_arg(&self) -> Arg<'_>;
}

pub(crate) trait MessageEnum {
    fn args(&self) -> Vec<Arg<'_>>;
}

pub(crate) struct ByteStream<'a> {
    pub backend: &'a Backend,
    pub bytes: std::collections::vec_deque::Drain<'a, u8>,
    pub fds: &'a mut VecDeque<OwnedFd>,
}

impl<'a> ByteStream<'a> {
    pub fn backend(&self) -> &Backend {
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
        Ok(crate::util::array_from_iterator_unchecked(self.read_n(N)?))
    }

    fn read_fd(&mut self) -> Result<OwnedFd, ParseError> {
        self.fds.pop_front().ok_or(ParseError::NoFd)
    }

    pub fn read_arg<T: OwnedArg>(&mut self) -> Result<T, ParseError> {
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
