//! Server-side EI protocol.
//!
//! Use [Listener] to create a socket, listening for clients creating a new
//! [Context].

use std::{
    io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::{UnixListener, UnixStream},
    },
    path::Path,
};

use crate::{Backend, ConnectionReadResult, Object, PendingRequestResult};

// Re-export generate bindings
pub use crate::eiproto_eis::*;

pub struct Listener {
    listener: UnixListener,
}

// TODO lockfile, unlink, etc.
impl Listener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let listener = UnixListener::bind(path)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    pub fn accept(&self) -> io::Result<Option<Context>> {
        match self.listener.accept() {
            Ok((socket, _)) => Ok(Some(Context::new(socket)?)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd {
        self.listener.as_fd()
    }
}

impl AsRawFd for Listener {
    fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }
}

#[derive(Clone, Debug)]
pub struct Context(pub(crate) Backend);

impl AsFd for Context {
    fn as_fd(&self) -> BorrowedFd {
        self.0.as_fd()
    }
}

impl AsRawFd for Context {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

impl Context {
    pub(crate) fn new(socket: UnixStream) -> io::Result<Self> {
        Ok(Self(Backend::new(socket, false)?))
    }

    /// Read any pending data on socket into buffer
    pub fn read(&self) -> io::Result<ConnectionReadResult> {
        self.0.read()
    }

    // XXX iterator
    pub fn pending_request(&self) -> Option<PendingRequestResult<Request>> {
        self.0.pending(Request::parse)
    }

    pub fn handshake(&self) -> handshake::Handshake {
        handshake::Handshake(Object::new(self.0.clone(), 0))
    }

    pub fn object_interface(&self, id: u64) -> Option<(String, u32)> {
        self.0.object_interface(id)
    }

    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.flush()
    }
}

#[doc(hidden)]
pub trait Interface: crate::Interface {}
