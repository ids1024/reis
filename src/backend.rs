use rustix::io::{Errno, IoSlice};
use std::{
    collections::{HashMap, VecDeque},
    env, io,
    os::unix::{
        io::{AsFd, BorrowedFd, OwnedFd},
        net::UnixStream,
    },
    sync::{Arc, Mutex},
};

use crate::{ei, eis, util, Arg, ByteStream, Header};

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
    objects: HashMap<u64, (String, u32)>,
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

impl AsFd for Backend {
    fn as_fd(&self) -> BorrowedFd {
        self.0.socket.as_fd()
    }
}

pub enum ConnectionReadResult {
    Read(usize),
    EOF,
}

pub enum PendingRequestResult<T> {
    Request(T),
    ProtocolError(String),
    InvalidObject(u64),
}

impl ConnectionReadResult {
    pub fn is_eof(&self) -> bool {
        matches!(self, Self::EOF)
    }
}

impl Backend {
    pub fn new(socket: UnixStream, client: bool) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        let next_id = if client { 1 } else { 0xff00000000000000 };
        let next_peer_id = if client { 0xff00000000000000 } else { 1 };
        let mut objects = HashMap::new();
        objects.insert(0, ("ei_handshake".to_string(), 1));
        Ok(Self(Arc::new(BackendInner {
            socket,
            client,
            state: Mutex::new(BackendState {
                next_id,
                next_peer_id,
                objects,
            }),
            read: Mutex::new(Buffer::default()),
            write: Mutex::new(Buffer::default()),
            debug: is_reis_debug(),
        })))
    }

    /// Read any pending data on socket into buffer
    pub fn read(&self) -> io::Result<ConnectionReadResult> {
        let mut read = self.0.read.lock().unwrap();

        // TODO read into read.buf with iov?
        let mut buf = [0; 2048];
        let mut total_count = 0;
        loop {
            match util::recv_with_fds(&self.0.socket, &mut buf, &mut read.fds) {
                Ok(0) if total_count == 0 => {
                    return Ok(ConnectionReadResult::EOF);
                }
                Ok(0) => {
                    return Ok(ConnectionReadResult::Read(total_count));
                }
                Ok(count) => {
                    read.buf.extend(&buf[0..count]);
                    total_count += count;
                }
                #[allow(unreachable_patterns)] // `WOULDBLOCK` and `AGAIN` typically equal
                Err(Errno::WOULDBLOCK | Errno::AGAIN) => {
                    return Ok(ConnectionReadResult::Read(total_count));
                }
                Err(err) => return Err(err.into()),
            };
        }
    }

    pub(crate) fn pending<T: crate::MessageEnum>(
        &self,
        parse: fn(u64, &str, u32, &mut crate::ByteStream) -> Result<T, crate::ParseError>,
    ) -> Option<PendingRequestResult<T>> {
        let mut read = self.0.read.lock().unwrap();
        if read.buf.len() >= 16 {
            let header_bytes = util::array_from_iterator_unchecked(read.buf.iter().copied());
            let header = Header::parse(header_bytes);
            if read.buf.len() < header.length as usize {
                return None;
            }
            if header.length < 16 {
                return Some(PendingRequestResult::ProtocolError(
                    "header length < 16".to_string(),
                ));
            }
            if let Some((interface, _version)) = self.object_interface(header.object_id) {
                let read = &mut *read;
                read.buf.drain(..16); // Remove header
                let mut bytes = ByteStream {
                    backend: self,
                    bytes: read.buf.drain(..header.length as usize - 16),
                    fds: &mut read.fds,
                };
                let request = parse(header.object_id, &interface, header.opcode, &mut bytes);
                if bytes.bytes.len() != 0 {
                    return Some(PendingRequestResult::ProtocolError(
                        "message length doesn't match header".to_string(),
                    ));
                }

                Some(match request {
                    Ok(request) => {
                        if self.0.debug {
                            self.print_msg(header.object_id, header.opcode, &request.args(), true);
                        }
                        PendingRequestResult::Request(request)
                    }
                    Err(err) => PendingRequestResult::ProtocolError(format!(
                        "failed to parse message: {:?}",
                        err
                    )),
                })
            } else {
                read.buf.drain(0..header.length as usize);
                Some(PendingRequestResult::InvalidObject(header.object_id))
            }
        } else {
            None
        }
    }

    pub fn new_id(&self, interface: String, version: u32) -> u64 {
        let mut state = self.0.state.lock().unwrap();
        let id = state.next_id;
        state.next_id += 1;
        state.objects.insert(id, (interface, version));
        id
    }

    fn new_peer_id(
        &self,
        id: u64,
        interface: String,
        version: u32,
    ) -> Result<(), crate::ParseError> {
        let mut state = self.0.state.lock().unwrap();
        if id < state.next_peer_id || (!self.0.client && id >= 0xff00000000000000) {
            return Err(crate::ParseError::InvalidId);
        }
        state.next_peer_id = id + 1;
        state.objects.insert(id, (interface, version));
        Ok(())
    }

    pub(crate) fn new_peer_object(
        &self,
        id: u64,
        interface: String,
        version: u32,
    ) -> Result<crate::Object, crate::ParseError> {
        self.new_peer_id(id, interface, version)?;
        Ok(crate::Object::new(self.clone(), id, self.0.client))
    }

    pub(crate) fn new_peer_interface<T: crate::Interface>(
        &self,
        id: u64,
        version: u32,
    ) -> Result<T, crate::ParseError> {
        Ok(self
            .new_peer_object(id, T::NAME.to_string(), version)?
            .downcast_unchecked())
    }

    pub fn remove_id(&self, id: u64) {
        self.0.state.lock().unwrap().objects.remove(&id);
    }

    // TODO avoid allocation? Would `Arc<String>` be better?
    pub fn object_interface(&self, id: u64) -> Option<(String, u32)> {
        self.0.state.lock().unwrap().objects.get(&id).cloned()
    }

    fn print_msg(&self, object_id: u64, opcode: u32, args: &[Arg], incoming: bool) {
        let interface = self
            .object_interface(object_id)
            .map(|x| x.0)
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let op_name = if self.0.client != incoming {
            eis::Request::op_name(&interface, opcode)
        } else {
            ei::Event::op_name(&interface, opcode)
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
            eprint!("{}", arg);
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

    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.write.lock().unwrap().flush_write(&self.0.socket)
    }
}

fn is_reis_debug() -> bool {
    env::var_os("REIS_DEBUG").map_or(false, |value| !value.is_empty())
}
