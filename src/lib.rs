#![forbid(unsafe_code)]

// TODO error type?
// TODO split up
// Implement handshake

use std::{env, os::unix::io::OwnedFd, path::PathBuf};

mod arg;
use arg::{Arg, OwnedArg};
mod connection;
pub use connection::{Connection, ConnectionReadResult, PendingRequestResult};
pub mod ei;
#[allow(unused_parens)]
mod eiproto_ei;
#[allow(unused_parens)]
mod eiproto_eis;
pub mod eis;
mod object;
use object::Object;
mod util;

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

#[derive(Debug)]
struct Message {
    header: Header,
    contents: Vec<u8>,
    // TODO fds?
}

trait Interface {
    const NAME: &'static str;
    const VERSION: u32;
    type Incoming;
}

struct ByteStream<'a> {
    connection: &'a Connection,
    bytes: &'a [u8],
    fds: &'a mut Vec<OwnedFd>,
}

impl<'a> ByteStream<'a> {
    fn connection(&self) -> &Connection {
        &self.connection
    }

    fn read_n(&mut self, n: usize) -> Option<&[u8]> {
        if self.bytes.len() >= n {
            let value;
            (value, self.bytes) = self.bytes.split_at(n);
            Some(value)
        } else {
            None
        }
    }

    fn read<const N: usize>(&mut self) -> Option<[u8; N]> {
        if self.bytes.len() >= N {
            let value;
            (value, self.bytes) = self.bytes.split_at(N);
            Some(value.try_into().unwrap())
        } else {
            None
        }
    }

    fn read_fd(&mut self) -> Option<OwnedFd> {
        if !self.fds.is_empty() {
            Some(self.fds.remove(0))
        } else {
            None
        }
    }

    fn read_arg<T: OwnedArg>(&mut self) -> Option<T> {
        T::parse(self)
    }
}
