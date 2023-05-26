#![forbid(unsafe_code)]

// TODO error type?
// TODO split up
// Implement handshake

use std::{env, os::unix::io::OwnedFd, path::PathBuf, string::FromUtf8Error, sync::Arc};

mod arg;
use arg::{Arg, OwnedArg};
mod backend;
use backend::Backend;
pub use backend::{ConnectionReadResult, PendingRequestResult}; // XXX types? names?
pub mod ei;
mod eiproto_ei;
mod eiproto_eis;
pub mod eis;
mod object;
use object::Object;
mod util;

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
    fn parse(bytes: &[u8]) -> Option<Self> {
        Some(Self {
            object_id: u64::from_ne_bytes(bytes[0..8].try_into().ok()?),
            length: u32::from_ne_bytes(bytes[8..12].try_into().ok()?),
            opcode: u32::from_ne_bytes(bytes[12..16].try_into().ok()?),
        })
    }

    /// Writes header into start of `buf`; panic if it has length less than 16
    fn write_at(&self, buf: &mut [u8]) {
        buf[0..8].copy_from_slice(&self.object_id.to_ne_bytes());
        buf[8..12].copy_from_slice(&self.length.to_ne_bytes());
        buf[12..16].copy_from_slice(&self.opcode.to_ne_bytes());
    }
}

#[doc(hidden)]
pub trait Interface: private::Sealed {
    const NAME: &'static str;
    const VERSION: u32;
    type Incoming;

    fn new_unchecked(object: Object) -> Self;
}

struct ByteStream<'a> {
    backend: &'a Arc<Backend>,
    bytes: &'a [u8],
    fds: &'a mut Vec<OwnedFd>,
}

impl<'a> ByteStream<'a> {
    fn backend(&self) -> &Arc<Backend> {
        self.backend
    }

    fn read_n(&mut self, n: usize) -> Result<&[u8], ParseError> {
        if self.bytes.len() >= n {
            let value;
            (value, self.bytes) = self.bytes.split_at(n);
            Ok(value)
        } else {
            Err(ParseError::EndOfMessage)
        }
    }

    fn read<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        if self.bytes.len() >= N {
            let value;
            (value, self.bytes) = self.bytes.split_at(N);
            Ok(value.try_into().unwrap())
        } else {
            Err(ParseError::EndOfMessage)
        }
    }

    fn read_fd(&mut self) -> Result<OwnedFd, ParseError> {
        if !self.fds.is_empty() {
            Ok(self.fds.remove(0))
        } else {
            Err(ParseError::NoFd)
        }
    }

    fn read_arg<T: OwnedArg>(&mut self) -> Result<T, ParseError> {
        T::parse(self)
    }
}

// TODO add detail, format for display
enum ParseError {
    EndOfMessage,
    Utf8,
    InvalidId,
    NoFd,
    InvalidOpcode,
    InvalidInterface,
    NoObject,
}

impl From<FromUtf8Error> for ParseError {
    fn from(_err: FromUtf8Error) -> Self {
        Self::Utf8
    }
}
