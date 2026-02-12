/// Backend
///
/// Handles socket reads/writes, byte/fd buffering, and calls into
/// serialization code to send/receive discrete typed message.
///
/// Also implements debug printing to stderr when `REIS_DEBUG` is set.
use rustix::io::{Errno, IoSlice};
use std::{
    collections::{HashMap, VecDeque},
    env, io,
    os::unix::{
        io::{AsFd, BorrowedFd, OwnedFd},
        net::UnixStream,
    },
    sync::{Arc, Mutex, Weak},
};

use crate::{
    ei, eis, util,
    wire::{self, Arg, ByteStream, Header, ParseError},
    Object,
};

#[derive(Debug, Default)]
struct Buffer {
    buf: VecDeque<u8>,
    fds: VecDeque<OwnedFd>,
}

impl Buffer {
    fn flush_write(&mut self, socket: &UnixStream) -> rustix::io::Result<()> {
        // TODO avoid allocation
        while !self.buf.is_empty() {
            let (slice1, slice2) = self.buf.as_slices();
            let iov = &[IoSlice::new(slice1), IoSlice::new(slice2)];
            let fds: Vec<_> = self.fds.iter().map(|x| x.as_fd()).collect();
            let written = util::send_with_fds(socket, iov, &fds)?;
            self.buf.drain(0..written).for_each(|_| {});
            self.fds.clear();
        }
        Ok(())
    }
}

#[derive(Debug)]
struct BackendState {
    next_id: u64,
    next_peer_id: u64,
    objects: HashMap<u64, Object>,
}

#[derive(Debug)]
struct BackendInner {
    socket: UnixStream,
    client: bool,
    state: Mutex<BackendState>,
    read: Mutex<Buffer>,
    write: Mutex<Buffer>,
    debug: bool,
}

// Used for both ei and eis
#[derive(Clone, Debug)]
pub struct Backend(Arc<BackendInner>);

#[derive(Clone, Debug)]
pub(crate) struct BackendWeak(Weak<BackendInner>);

impl BackendWeak {
    pub fn upgrade(&self) -> Option<Backend> {
        self.0.upgrade().map(Backend)
    }

    pub fn new_object(&self, interface: String, version: u32) -> Object {
        if let Some(backend) = self.upgrade() {
            backend.new_object(interface, version)
        } else {
            // If the backend is destroyed, object will be inert and id doesn't matter
            Object::for_new_id(self.clone(), u64::MAX, false, interface, version)
        }
    }

    pub fn remove_id(&self, id: u64) {
        if let Some(backend) = self.upgrade() {
            backend.remove_id(id);
        }
    }
}

impl AsFd for Backend {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.socket.as_fd()
    }
}

/// Pending message result.
#[derive(Debug)]
pub enum PendingRequestResult<T> {
    /// The message. Either an event or a request.
    Request(T),
    /// Wire format parse error.
    ParseError(ParseError),
    /// Invalid object ID.
    InvalidObject(u64),
}

impl Backend {
    /// Creates a [`Backend`] based on the given `socket`, and whether this is the `client`
    /// side or not.
    ///
    /// # Errors
    ///
    /// Will return `Err` if setting the socket to non-blocking mode fails.
    pub fn new(socket: UnixStream, client: bool) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        let next_id = if client { 1 } else { 0xff00_0000_0000_0000 };
        let next_peer_id = if client { 0xff00_0000_0000_0000 } else { 1 };
        let backend = Self(Arc::new(BackendInner {
            socket,
            client,
            state: Mutex::new(BackendState {
                next_id,
                next_peer_id,
                objects: HashMap::new(),
            }),
            read: Mutex::new(Buffer::default()),
            write: Mutex::new(Buffer::default()),
            debug: is_reis_debug(),
        }));
        let handshake =
            Object::for_new_id(backend.downgrade(), 0, client, "ei_handshake".to_owned(), 1);
        backend.0.state.lock().unwrap().objects.insert(0, handshake);
        Ok(backend)
    }

    pub(crate) fn downgrade(&self) -> BackendWeak {
        BackendWeak(Arc::downgrade(&self.0))
    }

    /// Reads any pending data on the socket into the backend's internal buffer.
    ///
    /// Returns `UnexpectedEof` if end-of-file is reached.
    pub fn read(&self) -> io::Result<usize> {
        let mut read = self.0.read.lock().unwrap();

        // TODO read into read.buf with iov?
        let mut buf = [0; 2048];
        let mut total_count = 0;
        loop {
            match util::recv_with_fds(&self.0.socket, &mut buf, &mut read.fds) {
                Ok(0) if total_count == 0 => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "unexpected EOF reading ei socket",
                    ));
                }
                #[allow(unreachable_patterns)] // `WOULDBLOCK` and `AGAIN` typically equal
                Ok(0) | Err(Errno::WOULDBLOCK | Errno::AGAIN) => {
                    return Ok(total_count);
                }
                Ok(count) => {
                    read.buf.extend(&buf[0..count]);
                    total_count += count;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    /// Returns a message that is readily available.
    pub(crate) fn pending<T: wire::MessageEnum>(
        &self,
        parse: fn(Object, u32, &mut ByteStream) -> Result<T, ParseError>,
    ) -> Option<PendingRequestResult<T>> {
        let mut read = self.0.read.lock().unwrap();
        if read.buf.len() >= 16 {
            let header_bytes = util::array_from_iterator_unchecked(read.buf.iter().copied());
            let header = Header::parse(header_bytes);
            if read.buf.len() < header.length as usize {
                return None;
            }
            if header.length < 16 {
                return Some(PendingRequestResult::ParseError(ParseError::HeaderLength(
                    header.length,
                )));
            }
            if let Some(object) = self.object_for_id(header.object_id) {
                let read = &mut *read;
                read.buf.drain(..16); // Remove header
                let mut bytes = ByteStream {
                    backend: self,
                    bytes: read.buf.drain(..header.length as usize - 16),
                    fds: &mut read.fds,
                };
                let request = match parse(object, header.opcode, &mut bytes) {
                    Ok(request) => request,
                    Err(err) => return Some(PendingRequestResult::ParseError(err)),
                };

                if bytes.bytes.len() != 0 {
                    return Some(PendingRequestResult::ParseError(ParseError::MessageLength(
                        header.length + bytes.bytes.len() as u32,
                        header.length,
                    )));
                }

                if self.0.debug {
                    self.print_msg(header.object_id, header.opcode, &request.args(), true);
                }
                Some(PendingRequestResult::Request(request))
            } else {
                read.buf.drain(0..header.length as usize);
                Some(PendingRequestResult::InvalidObject(header.object_id))
            }
        } else {
            None
        }
    }

    pub fn new_object(&self, interface: String, version: u32) -> Object {
        let mut state = self.0.state.lock().unwrap();

        let id = state.next_id;
        state.next_id += 1;

        let object = Object::for_new_id(self.downgrade(), id, self.0.client, interface, version);
        state.objects.insert(id, object.clone());
        object
    }

    pub(crate) fn new_peer_object(
        &self,
        id: u64,
        interface: String,
        version: u32,
    ) -> Result<crate::Object, ParseError> {
        let mut state = self.0.state.lock().unwrap();

        if id < state.next_peer_id || (!self.0.client && id >= 0xff00_0000_0000_0000) {
            return Err(ParseError::InvalidId(id));
        }
        state.next_peer_id = id + 1;

        let object =
            crate::Object::for_new_id(self.downgrade(), id, self.0.client, interface, version);
        state.objects.insert(id, object.clone());
        Ok(object)
    }

    pub(crate) fn new_peer_interface<T: crate::wire::Interface>(
        &self,
        id: u64,
        version: u32,
    ) -> Result<T, ParseError> {
        Ok(self
            .new_peer_object(id, T::NAME.to_owned(), version)?
            .downcast_unchecked())
    }

    pub fn remove_id(&self, id: u64) {
        self.0.state.lock().unwrap().objects.remove(&id);
    }

    pub fn object_for_id(&self, id: u64) -> Option<Object> {
        self.0.state.lock().unwrap().objects.get(&id).cloned()
    }

    pub(crate) fn has_object_for_id(&self, id: u64) -> bool {
        self.0.state.lock().unwrap().objects.contains_key(&id)
    }

    fn print_msg(&self, object_id: u64, opcode: u32, args: &[Arg], incoming: bool) {
        let object = self.object_for_id(object_id);
        let interface = object.as_ref().map_or("UNKNOWN", |x| x.interface());
        let op_name = if self.0.client == incoming {
            ei::Event::op_name(interface, opcode)
        } else {
            eis::Request::op_name(interface, opcode)
        }
        .unwrap_or("UNKNOWN");
        if incoming {
            eprint!(" -> ");
        }
        eprint!("{interface}@{object_id:x}.{op_name}(");
        let mut first = true;
        for arg in args {
            if !first {
                eprint!(", ");
            }
            first = false;
            eprint!("{arg}");
        }
        eprintln!(")");
    }

    pub fn request(&self, object_id: u64, opcode: u32, args: &[Arg]) {
        if self.0.debug {
            self.print_msg(object_id, opcode, args, false);
        }

        let mut write = self.0.write.lock().unwrap();

        let start_len = write.buf.len();

        // Leave space for header
        write.buf.extend([0; 16]);

        // Write arguments
        for arg in args {
            let write = &mut *write;
            arg.write(&mut write.buf, &mut write.fds);
        }

        // Write header now we know the length
        let header = Header {
            object_id,
            length: (write.buf.len() - start_len) as u32,
            opcode,
        };
        for (i, b) in header.as_bytes().enumerate() {
            write.buf[start_len + i] = b;
        }
    }

    /// Sends buffered messages.
    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.write.lock().unwrap().flush_write(&self.0.socket)
    }

    /// Shutdown read end of socket, so all future reads will return EOF
    pub(crate) fn shutdown_read(&self) {
        let _ = self.0.socket.shutdown(std::net::Shutdown::Read);
    }
}

fn is_reis_debug() -> bool {
    env::var_os("REIS_DEBUG").is_some_and(|value| !value.is_empty())
}
