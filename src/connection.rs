use rustix::io::Errno;
use std::{
    collections::HashMap,
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
    buf: Vec<u8>,
    fds: Vec<OwnedFd>,
}

#[derive(Debug)]
struct ConnectionState {
    next_id: u64,
    next_peer_id: u64,
    objects: HashMap<u64, (String, u32)>,
}

// Ref-counted; shared for both ei and eis
#[derive(Debug)]
pub struct ConnectionInner {
    socket: UnixStream,
    client: bool,
    state: Mutex<ConnectionState>,
    read: Mutex<Buffer>,
}

impl AsFd for ConnectionInner {
    fn as_fd(&self) -> BorrowedFd {
        self.socket.as_fd()
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

impl ConnectionInner {
    pub fn new(socket: UnixStream, client: bool) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        let next_id = if client { 1 } else { 0xff00000000000000 };
        let next_peer_id = if client { 0xff00000000000000 } else { 1 };
        let mut objects = HashMap::new();
        objects.insert(0, ("ei_handshake".to_string(), 1));
        Ok(Self {
            socket,
            client,
            state: Mutex::new(ConnectionState {
                next_id,
                next_peer_id,
                objects,
            }),
            read: Mutex::new(Buffer {
                buf: Vec::new(),
                fds: Vec::new(),
            }),
        })
    }

    /// Read any pending data on socket into buffer
    pub fn read(&self) -> io::Result<ConnectionReadResult> {
        let mut read = self.read.lock().unwrap();

        let mut buf = [0; 2048];
        let mut total_count = 0;
        loop {
            match util::recv_with_fds(&self.socket, &mut buf, &mut read.fds) {
                Ok(0) if total_count == 0 => {
                    return Ok(ConnectionReadResult::EOF);
                }
                Ok(0) => {
                    return Ok(ConnectionReadResult::Read(total_count));
                }
                Ok(count) => {
                    read.buf.extend_from_slice(&buf[0..count]);
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
        self: &Arc<Self>,
        parse: fn(&str, u32, &mut crate::ByteStream) -> Result<T, crate::ParseError>,
    ) -> Option<PendingRequestResult<T>> {
        let mut read = self.read.lock().unwrap();
        if read.buf.len() >= 16 {
            let header = Header::parse(&read.buf).unwrap();
            if read.buf.len() < header.length as usize {
                return None;
            }
            if header.length < 16 {
                return Some(PendingRequestResult::ProtocolError("header length < 16"));
            }
            if let Some((interface, _version)) = self.object_interface(header.object_id) {
                let read = &mut *read;
                let mut bytes = ByteStream {
                    connection: self,
                    bytes: &read.buf[16..header.length as usize],
                    fds: &mut read.fds,
                };
                let request = parse(&interface, header.opcode, &mut bytes);
                if !bytes.bytes.is_empty() {
                    return Some(PendingRequestResult::ProtocolError(
                        "message length doesn't match header",
                    ));
                }

                // XXX inefficient
                read.buf.drain(0..header.length as usize);

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
        let mut state = self.state.lock().unwrap();
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
        let mut state = self.state.lock().unwrap();
        if id < state.next_peer_id || (!self.client && id >= 0xff00000000000000) {
            return Err(crate::ParseError::InvalidId);
        }
        state.next_peer_id = id + 1;
        state.objects.insert(id, (interface, version));
        Ok(())
    }

    pub(crate) fn new_peer_object(
        self: &Arc<Self>,
        id: u64,
        interface: String,
        version: u32,
    ) -> Result<crate::Object, crate::ParseError> {
        self.new_peer_id(id, interface, version)?;
        Ok(crate::Object::new(self.clone(), id))
    }

    pub(crate) fn new_peer_interface<T: crate::Interface>(
        self: &Arc<Self>,
        id: u64,
        version: u32,
    ) -> Result<T, crate::ParseError> {
        Ok(T::downcast_unchecked(self.new_peer_object(
            id,
            T::NAME.to_string(),
            version,
        )?))
    }

    pub fn remove_id(&self, id: u64) {
        self.state.lock().unwrap().objects.remove(&id);
    }

    // TODO avoid allocation? Would `Arc<String>` be better?
    pub fn object_interface(&self, id: u64) -> Option<(String, u32)> {
        self.state.lock().unwrap().objects.get(&id).cloned()
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
        util::send_with_fds(&self.socket, &buf, &fds)
    }
}
