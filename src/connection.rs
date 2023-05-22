use rustix::io::Errno;
use std::{
    collections::HashMap,
    io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
        net::UnixStream,
    },
    sync::{Arc, Mutex},
};

use crate::{eis, util, Arg, ByteStream, Header, Object};

#[derive(Debug)]
struct Buffer {
    buf: Vec<u8>,
    fds: Vec<OwnedFd>,
}

#[derive(Debug)]
struct ConnectionState {
    next_id: u64,
    objects: HashMap<u64, &'static str>,
}

#[derive(Debug)]
struct ConnectionInner {
    socket: UnixStream,
    // TODO distinguish at type level?
    client: bool,
    state: Mutex<ConnectionState>,
    read: Mutex<Buffer>,
}

#[derive(Clone, Debug)]
pub struct Connection(Arc<ConnectionInner>);

impl AsFd for Connection {
    fn as_fd(&self) -> BorrowedFd {
        self.0.socket.as_fd()
    }
}

impl AsRawFd for Connection {
    fn as_raw_fd(&self) -> RawFd {
        self.0.socket.as_raw_fd()
    }
}

pub enum ConnectionReadResult {
    Read(usize),
    EOF,
}

pub enum PendingRequestResult {
    Request(eis::Request),
    ProtocolError(&'static str),
    InvalidObject(u64),
}

impl ConnectionReadResult {
    pub fn is_eof(&self) -> bool {
        matches!(self, Self::EOF)
    }
}

impl Connection {
    pub(crate) fn new(socket: UnixStream, client: bool) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        let next_id = if client { 1 } else { 0xff00000000000000 };
        let mut objects = HashMap::new();
        objects.insert(0, "ei_handshake");
        Ok(Self(Arc::new(ConnectionInner {
            socket,
            client,
            state: Mutex::new(ConnectionState { next_id, objects }),
            read: Mutex::new(Buffer {
                buf: Vec::new(),
                fds: Vec::new(),
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

    // XXX seperate type for ei
    // XXX iterator
    pub fn eis_pending_request(&self) -> Option<PendingRequestResult> {
        let mut read = self.0.read.lock().unwrap();
        if read.buf.len() >= 16 {
            let header = Header::parse(&read.buf).unwrap();
            if read.buf.len() < header.length as usize {
                return None;
            }
            if header.length < 16 {
                return Some(PendingRequestResult::ProtocolError("header length < 16"));
            }
            if let Some(interface) = self.object_interface(header.object_id) {
                let read = &mut *read;
                let mut bytes = ByteStream {
                    connection: self,
                    bytes: &read.buf[16..header.length as usize],
                    fds: &mut read.fds,
                };
                let request = eis::Request::parse(interface, header.opcode, &mut bytes);
                if bytes.bytes.len() != 0 {
                    return Some(PendingRequestResult::ProtocolError(
                        "message length doesn't match header",
                    ));
                }

                // XXX inefficient
                for i in 0..header.length as usize {
                    read.buf.remove(0);
                }

                if let Some(request) = request {
                    Some(PendingRequestResult::Request(request))
                } else {
                    Some(PendingRequestResult::ProtocolError(
                        "failed to parse request",
                    ))
                }
            } else {
                Some(PendingRequestResult::InvalidObject(header.object_id))
            }
        } else {
            None
        }
    }

    // TODO: can't have iterator over messages without knowing what fds belong to what message...

    // XXX distinguish ei/eis connection types
    pub fn eis_handshake(&self) -> crate::eis::handshake::Handshake {
        eis::handshake::Handshake(Object::new(self.clone(), 0))
    }

    // TODO send return value? send more?
    // TODO buffer nonblocking output?
    // XXX don't allow write from multiple threads without lock
    fn send(&self, data: &[u8], fds: &[BorrowedFd]) -> rustix::io::Result<()> {
        util::send_with_fds(&self.0.socket, data, fds)
    }

    pub(crate) fn new_id(&self, interface: &'static str) -> u64 {
        let mut state = self.0.state.lock().unwrap();
        let id = state.next_id;
        state.next_id += 1;
        state.objects.insert(id, interface);
        id
    }

    pub(crate) fn remove_id(&self, id: u64) {
        self.0.state.lock().unwrap().objects.remove(&id);
    }

    pub fn object_interface(&self, id: u64) -> Option<&'static str> {
        self.0.state.lock().unwrap().objects.get(&id).copied()
    }

    pub(crate) fn request(
        &self,
        object_id: u64,
        opcode: u32,
        args: &[Arg],
    ) -> rustix::io::Result<()> {
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
        self.send(&buf, &fds)
    }
}
