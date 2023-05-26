use std::{
    io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::UnixStream,
    },
    sync::Arc,
};

use crate::{ConnectionInner, ConnectionReadResult, Object, PendingRequestResult};

// Re-export generate bindings
pub use crate::eiproto_ei::*;

#[derive(Clone, Debug)]
pub struct Connection(pub(crate) Arc<ConnectionInner>);

impl AsFd for Connection {
    fn as_fd(&self) -> BorrowedFd {
        self.0.as_fd()
    }
}

impl AsRawFd for Connection {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

impl Connection {
    // TODO way to connect
    pub fn new(socket: UnixStream) -> io::Result<Self> {
        Ok(Self(Arc::new(ConnectionInner::new(socket, false)?)))
    }

    /// Read any pending data on socket into buffer
    pub fn read(&self) -> io::Result<ConnectionReadResult> {
        self.0.read()
    }

    // XXX iterator
    pub fn pending_event(&self) -> Option<PendingRequestResult<Event>> {
        self.0.pending(Event::parse)
    }

    pub fn handshake(&self) -> handshake::Handshake {
        handshake::Handshake(Object::new(self.0.clone(), 0))
    }

    pub fn object_interface(&self, id: u64) -> Option<&'static str> {
        self.0.object_interface(id)
    }
}

#[doc(hidden)]
pub trait Interface: crate::Interface {}
