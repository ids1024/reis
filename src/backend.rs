use rustix::io::Errno;
use std::{
    collections::{HashMap, VecDeque},
    io,
    os::unix::{
        io::{AsFd, BorrowedFd, OwnedFd},
        net::UnixStream,
    },
    sync::{Arc, Mutex},
};

use crate::{util, Arg, ByteStream, Header};

#[derive(Debug)]
struct Buffer {
    buf: VecDeque<u8>,
    fds: VecDeque<OwnedFd>,
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
    ProtocolError(&'static str),
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
            read: Mutex::new(Buffer {
                buf: VecDeque::new(),
                fds: VecDeque::new(),
            }),
        })))
    }

    /// Read any pending data on socket into buffer
    pub fn read(&self) -> io::Result<ConnectionReadResult> {
        let mut read = self.0.read.lock().unwrap();

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

    pub(crate) fn pending<T>(
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
                return Some(PendingRequestResult::ProtocolError("header length < 16"));
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
                        "message length doesn't match header",
                    ));
                }

                if let Ok(request) = request {
                    Some(PendingRequestResult::Request(request))
                } else {
                    // XXX handle specific error
                    Some(PendingRequestResult::ProtocolError(
                        "failed to parse request",
                    ))
                }
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
        Ok(crate::Object::new(self.clone(), id))
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

    // TODO send return value? send more?
    // TODO buffer nonblocking output?
    // XXX don't allow write from multiple threads without lock
    pub fn request(&self, object_id: u64, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
        let interface = self.object_interface(object_id).map(|x| x.0);
        println!(
            "Request {:?} {} {}: {:?}",
            interface, object_id, opcode, args
        );
        // Leave space for header
        let mut buf = vec![0; 16];
        let mut fds = vec![];
        for arg in args {
            arg.write(&mut buf, &mut fds);
        }
        let header = Header {
            object_id,
            length: buf.len() as u32,
            opcode,
        };
        header.write_at(&mut buf);
        util::send_with_fds(&self.0.socket, &buf, &fds)
    }
}
