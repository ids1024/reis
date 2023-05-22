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
    objects: HashMap<u64, &'static str>,
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
        let mut objects = HashMap::new();
        objects.insert(0, "ei_handshake");
        Ok(Self {
            socket,
            client,
            state: Mutex::new(ConnectionState { next_id, objects }),
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
        parse: fn(&'static str, u32, &mut crate::ByteStream) -> Option<T>,
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
            if let Some(interface) = self.object_interface(header.object_id) {
                let read = &mut *read;
                let mut bytes = ByteStream {
                    connection: self,
                    bytes: &read.buf[16..header.length as usize],
                    fds: &mut read.fds,
                };
                let request = parse(interface, header.opcode, &mut bytes);
                if bytes.bytes.len() != 0 {
                    return Some(PendingRequestResult::ProtocolError(
                        "message length doesn't match header",
                    ));
                }

                // XXX inefficient
                read.buf.drain(0..header.length as usize);

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

    pub fn new_id(&self, interface: &'static str) -> u64 {
        let mut state = self.state.lock().unwrap();
        let id = state.next_id;
        state.next_id += 1;
        state.objects.insert(id, interface);
        id
    }

    pub fn remove_id(&self, id: u64) {
        self.state.lock().unwrap().objects.remove(&id);
    }

    pub fn object_interface(&self, id: u64) -> Option<&'static str> {
        self.state.lock().unwrap().objects.get(&id).copied()
    }

    // TODO send return value? send more?
    // TODO buffer nonblocking output?
    // XXX don't allow write from multiple threads without lock
    pub fn request(&self, object_id: u64, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
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
