//! EI client implementation.
//!
//! Create a connection over a Unix socket with [`Context`].
//!
//! Client-side protocol bindings are exported here, and they consist of interface proxies (like
//! [`device::Device`]) and event enums (like [`device::Event`]).

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

/// A connection, seen from the client side.
#[derive(Clone, Debug)]
pub struct Context(pub(crate) Backend);

impl AsFd for Context {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for Context {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

impl Context {
    /// Creates a `Context` from a `UnixStream`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::os::unix::net::UnixStream;
    /// use reis::ei::Context;
    ///
    /// let socket = UnixStream::connect("/example/path").unwrap();
    /// // Or receive from, for example, the RemoteDesktop XDG desktop protal.
    ///
    /// let context = Context::new(socket).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Will return `Err` if setting the socket to non-blocking mode fails.
    pub fn new(socket: UnixStream) -> io::Result<Self> {
        Ok(Self(Backend::new(socket, true)?))
    }

    /// Connects to a socket based on the `LIBEI_SOCKET` environment variable, and creates
    /// a `Context` from it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use reis::ei::Context;
    ///
    /// let context = Context::connect_to_env().expect("Shouldn't error").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Will return `Err` if the resolved socket path cannot be connected to or if
    /// [`Context::new`] fails.
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

    /// Reads any pending data on the socket into the internal buffer.
    ///
    /// Returns `UnexpectedEof` if end-of-file is reached.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an I/O error.
    pub fn read(&self) -> io::Result<usize> {
        self.0.read()
    }

    /// Returns an event that is readily available.
    // XXX iterator
    pub fn pending_event(&self) -> Option<PendingRequestResult<Event>> {
        self.0.pending(Event::parse)
    }

    /// Returns the interface proxy for the `ei_handshake` object.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // infallible; Backend always creates ei_handshake object at 0
    pub fn handshake(&self) -> handshake::Handshake {
        self.0.object_for_id(0).unwrap().downcast_unchecked()
    }

    /// Sends buffered messages. Call after you're finished with sending requests.
    ///
    /// # Errors
    ///
    /// An error will be returned if sending the buffered messages fails.
    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.flush()
    }
}

#[doc(hidden)]
pub trait Interface: crate::wire::Interface {}
