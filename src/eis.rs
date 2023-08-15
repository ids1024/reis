//! Server-side EI protocol.
//!
//! Use [Listener] to create a socket, listening for clients creating a new
//! [Context].

use std::{
    env, io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
};

use crate::{util, Backend, ConnectionReadResult, PendingRequestResult};

// Re-export generate bindings
pub use crate::eiproto_eis::*;

pub struct Listener {
    listener: util::UnlinkOnDrop<UnixListener>,
    _lock: Option<util::LockFile>,
}

impl Listener {
    // TODO Use a lock here
    pub fn bind(path: &Path) -> io::Result<Self> {
        Self::bind_inner(PathBuf::from(path), None)
    }

    // XXX result type?
    // Error if XDG_RUNTIME_DIR not set?
    pub fn bind_auto() -> io::Result<Option<Self>> {
        let xdg_dir = if let Some(var) = env::var_os("XDG_RUNTIME_DIR") {
            PathBuf::from(var)
        } else {
            return Ok(None);
        };
        for i in 1..33 {
            let lock_path = xdg_dir.join(format!("eis-{i}.lock"));
            let Some(lock_file) = util::LockFile::new(lock_path)? else {
                // Already locked, continue to next number
                continue;
            };
            let sock_path = xdg_dir.join(format!("eis-{i}"));
            return Self::bind_inner(sock_path, Some(lock_file)).map(Some);
        }
        Ok(None)
    }

    fn bind_inner(path: PathBuf, lock: Option<util::LockFile>) -> io::Result<Self> {
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        let listener = util::UnlinkOnDrop::new(listener, path);
        Ok(Self {
            listener,
            _lock: lock,
        })
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
        self.0.object_for_id(0).unwrap().downcast_unchecked()
    }

    /*
    pub fn object_interface(&self, id: u64) -> Option<(String, u32)> {
        self.0.object_interface(id)
    }
    */

    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.flush()
    }
}

#[doc(hidden)]
pub trait Interface: crate::Interface {}
