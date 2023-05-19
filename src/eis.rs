use std::{
    io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::UnixListener,
    },
    path::Path,
};

use crate::Connection;

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

    pub fn accept(&self) -> io::Result<Option<super::Connection>> {
        match self.listener.accept() {
            Ok((socket, _)) => Ok(Some(Connection::new(socket, false)?)),
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
