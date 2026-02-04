//! EI server (EIS) implementation.
//!
//! Use [`Listener`] to create a socket, listening for clients creating a new [`Context`]. See also the example for creating a Unix socket pair in [`Context::new`].
//!
//! Server-side protocol bindings are exported here, and they consist of interface proxies (like
//! [`device::Device`]) and request enums (like [`device::Request`]).

use std::{
    env, fmt, fs, io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, RawFd},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
};

use crate::{util, wire::Backend, PendingRequestResult};

// Re-export generate bindings
pub use crate::eiproto_eis::*;

/// Error returned from [`Listener::bind_auto`]
#[derive(Debug)]
pub enum BindError {
    /// The Environment variable `XDG_RUNTIME_DIR` is not set
    RuntimeDirNotSet,
    /// IO error
    Io(io::Error),
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeDirNotSet => write!(
                f,
                "environment variable XDG_RUNTIME_DIR is not set or invalid"
            ),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<io::Error> for BindError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl std::error::Error for BindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RuntimeDirNotSet => None,
            Self::Io(err) => Some(err),
        }
    }
}

/// EIS listener in a Unix socket.
#[derive(Debug)]
pub struct Listener {
    listener: util::UnlinkOnDrop<UnixListener>,
    _lock: Option<util::LockFile>,
}

impl Listener {
    // TODO Use a lock here
    /// Listens on a specific path.
    ///
    /// # Errors
    ///
    /// Will return `Err` if binding to the given path or setting the socket to
    /// non-blocking mode fails.
    pub fn bind(path: &Path) -> io::Result<Self> {
        Self::bind_inner(PathBuf::from(path), None)
    }

    /// Listens on a file in `XDG_RUNTIME_DIR`.
    ///
    /// # Errors
    ///
    /// Will return `Err` if a lock file cannot be locked, binding to the generated path
    /// fails or setting the socket to non-blocking mode fails.
    pub fn bind_auto() -> Result<Self, BindError> {
        let xdg_dir = if let Some(var) = env::var_os("XDG_RUNTIME_DIR") {
            PathBuf::from(var)
        } else {
            return Err(BindError::RuntimeDirNotSet);
        };
        for i in 0.. {
            let lock_path = xdg_dir.join(format!("eis-{i}.lock"));
            let Some(lock_file) = util::LockFile::new(lock_path)? else {
                // Already locked, continue to next number
                continue;
            };
            let sock_path = xdg_dir.join(format!("eis-{i}"));
            if sock_path.try_exists()? {
                fs::remove_file(&sock_path)?;
            }
            return Ok(Self::bind_inner(sock_path, Some(lock_file))?);
        }
        // Should never be reached
        Err(BindError::RuntimeDirNotSet)
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

    /// Get the path to the listener socket
    #[must_use]
    pub fn path(&self) -> &Path {
        util::UnlinkOnDrop::path(&self.listener)
    }

    /// Accepts a connection from a client. Returns `Ok(Some(_)` if an incoming connection is ready, and `Ok(None)` if there is no connection ready (would block).
    ///
    /// # Errors
    ///
    /// Will return `Err` if [`Context::new`] fails.
    pub fn accept(&self) -> io::Result<Option<Context>> {
        match self.listener.accept() {
            Ok((socket, _)) => Ok(Some(Context::new(socket)?)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.listener.as_fd()
    }
}

impl AsRawFd for Listener {
    fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }
}

/// A connection, seen from the server side.
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
    /// ```
    /// use std::os::unix::net::UnixStream;
    /// use reis::eis::Context;
    ///
    /// let (a, b) = UnixStream::pair().unwrap();
    /// let context = Context::new(a).unwrap();
    ///
    /// // Pass the `b` file descriptor to implement the RemoteDesktop XDG desktop portal
    /// ```
    ///
    /// # Errors
    ///
    /// Will return `Err` if setting the socket to non-blocking mode fails.
    pub fn new(socket: UnixStream) -> io::Result<Self> {
        Ok(Self(Backend::new(socket, false)?))
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

    /// Returns a request that is readily available.
    // XXX iterator
    pub fn pending_request(&self) -> Option<PendingRequestResult<Request>> {
        self.0.pending(Request::parse)
    }

    /// Returns the interface proxy for the `ei_handshake` object.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // infallible; Backend always creates ei_handshake object at 0
    pub fn handshake(&self) -> handshake::Handshake {
        self.0.object_for_id(0).unwrap().downcast_unchecked()
    }

    /// Sends buffered messages. Call after you're finished with sending events.
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
