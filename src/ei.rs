use std::{
    env, io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::UnixStream,
    },
    path::PathBuf,
};

use crate::{wire::Backend, PendingRequestResult};

// Re-export generate bindings
pub use crate::eiproto_ei::*;

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
    // TODO way to connect
    pub fn new(socket: UnixStream) -> io::Result<Self> {
        Ok(Self(Backend::new(socket, true)?))
    }

    pub fn connect_to_env() -> io::Result<Option<Self>> {
        let Some(path) = env::var_os("LIBEI_SOCKET") else {
            // XXX return error type
            return Ok(None);
        };
        let path = PathBuf::from(path);
        let path = if path.is_relative() {
            let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") else {
                // XXX return not found
                return Ok(None);
            };
            let mut new_path = PathBuf::from(runtime_dir);
            new_path.push(path);
            new_path
        } else {
            path
        };
        let socket = UnixStream::connect(path)?;
        Self::new(socket).map(Some)
    }

    /// Read any pending data on socket into buffer
    pub fn read(&self) -> io::Result<usize> {
        self.0.read()
    }

    // XXX iterator
    pub fn pending_event(&self) -> Option<PendingRequestResult<Event>> {
        self.0.pending(Event::parse)
    }

    pub fn handshake(&self) -> handshake::Handshake {
        self.0.object_for_id(0).unwrap().downcast_unchecked()
    }

    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.flush()
    }
}

#[doc(hidden)]
pub trait Interface: crate::wire::Interface {}
