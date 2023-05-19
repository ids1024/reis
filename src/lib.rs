#![forbid(unsafe_code)]

// TODO error type?
// TODO split up
// Implement handshake

use rustix::{
    io::{retry_on_intr, IoSlice, IoSliceMut},
    net,
};
use std::{
    collections::HashMap,
    env, io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

#[allow(unused_parens)]
pub mod ei {
    include!("eiproto_ei.rs");
}

#[allow(unused_parens)]
pub mod eis {
    use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd};

    pub struct Listener {
        listener: super::UnixListener,
    }

    // TODO lockfile, unlink, etc.
    impl Listener {
        pub fn bind(path: &super::Path) -> std::io::Result<Self> {
            let listener = super::UnixListener::bind(path)?;
            listener.set_nonblocking(true)?;
            Ok(Self { listener })
        }

        pub fn accept(&self) -> std::io::Result<Option<super::Connection>> {
            match self.listener.accept() {
                Ok((socket, _)) => Ok(Some(super::Connection::new(socket, false)?)),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
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

    include!("eiproto_eis.rs");
}

// TODO Listener?
// TODO versioning?

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

impl Connection {
    fn new(socket: UnixStream, client: bool) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        let next_id = if client { 1 } else { 0xff00000000000000 };
        let mut objects = HashMap::new();
        objects.insert(0, "ei_handshake");
        Ok(Self(Arc::new(ConnectionInner {
            socket,
            client,
            state: Mutex::new(ConnectionState { next_id, objects }),
        })))
    }

    // XXX distinguish ei/eis connection types
    pub fn eis_handshake(&self) -> eis::handshake::Handshake {
        eis::handshake::Handshake(Object::new(self.clone(), 0))
    }

    // TODO send return value? send more?
    // TODO buffer nonblocking output?
    // XXX don't allow write from multiple threads without lock
    fn send(&self, data: &[u8], fds: &[BorrowedFd]) -> rustix::io::Result<()> {
        let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(fds.len()))];
        let mut cmsg_buffer = net::SendAncillaryBuffer::new(&mut cmsg_space);
        cmsg_buffer.push(net::SendAncillaryMessage::ScmRights(&fds));
        retry_on_intr(|| {
            net::sendmsg_noaddr(
                &self.0.socket,
                &[IoSlice::new(data)],
                &mut cmsg_buffer,
                net::SendFlags::empty(),
            )
        })?;
        Ok(())
    }

    // XXX pub
    pub fn recv(&self, buf: &mut [u8], fds: &mut Vec<OwnedFd>) -> rustix::io::Result<usize> {
        const MAX_FDS: usize = 32;

        let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(MAX_FDS))];
        let mut cmsg_buffer = net::RecvAncillaryBuffer::new(&mut cmsg_space);
        let response = retry_on_intr(|| {
            net::recvmsg(
                &self.0.socket,
                &mut [IoSliceMut::new(buf)],
                &mut cmsg_buffer,
                net::RecvFlags::empty(),
            )
        })?;
        fds.extend(
            cmsg_buffer
                .drain()
                .filter_map(|msg| match msg {
                    net::RecvAncillaryMessage::ScmRights(fds) => Some(fds),
                    _ => None,
                })
                .flatten(),
        );
        Ok(response.bytes)
    }

    fn new_id(&self, interface: &'static str) -> u64 {
        let mut state = self.0.state.lock().unwrap();
        let id = state.next_id;
        state.next_id += 1;
        state.objects.insert(id, interface);
        id
    }

    pub fn object_interface(&self, id: u64) -> Option<&'static str> {
        self.0.state.lock().unwrap().objects.get(&id).copied()
    }

    fn request(&self, object_id: u64, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
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

// XXX
// Want to fallback to higher number if exists, on server?
// create on server, not client.
pub fn default_socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}

// XXX pub
#[derive(Debug)]
pub struct Header {
    pub object_id: u64,
    pub length: u32,
    pub opcode: u32,
}

impl Header {
    // XXX pub
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        Some(Self {
            object_id: u64::from_ne_bytes(bytes[0..8].try_into().ok()?),
            length: u32::from_ne_bytes(bytes[8..12].try_into().ok()?),
            opcode: u32::from_ne_bytes(bytes[12..16].try_into().ok()?),
        })
    }

    /// Writes header into start of `buf`; panic if it has length less than 16
    fn write_at(&self, buf: &mut [u8]) {
        buf[0..8].copy_from_slice(&self.object_id.to_ne_bytes());
        buf[8..12].copy_from_slice(&self.length.to_ne_bytes());
        buf[12..16].copy_from_slice(&self.opcode.to_ne_bytes());
    }
}

#[derive(Debug)]
struct Message {
    header: Header,
    contents: Vec<u8>,
    // TODO fds?
}

#[derive(Debug)]
enum Arg<'a> {
    Uint32(u32),
    Int32(i32),
    Uint64(u64),
    Int64(i64),
    Float(f32),
    Fd(BorrowedFd<'a>),
    String(&'a str),
    NewId(u64),
    Id(u64),
}

impl<'a> Arg<'a> {
    fn write(&self, buf: &mut Vec<u8>, fds: &mut Vec<BorrowedFd<'a>>) {
        match self {
            Arg::Uint32(value) => buf.extend(value.to_ne_bytes()),
            Arg::Int32(value) => buf.extend(value.to_ne_bytes()),
            Arg::Uint64(value) => buf.extend(value.to_ne_bytes()),
            Arg::Int64(value) => buf.extend(value.to_ne_bytes()),
            Arg::Float(value) => buf.extend(value.to_ne_bytes()),
            Arg::Fd(value) => fds.push(*value),
            Arg::String(value) => {
                // Write 32-bit length, including NUL
                let len = value.len() as u32 + 1;
                buf.extend(len.to_ne_bytes());
                // Write contents of string, as UTF-8
                buf.extend(value.as_bytes());
                // Add NUL terminator
                buf.push(b'\0');
                // Pad to multiple of 32 bits
                if len % 4 != 0 {
                    buf.extend((0..4 - (len % 4)).map(|_| b'\0'));
                }
            }
            Arg::NewId(value) => buf.extend(value.to_ne_bytes()),
            Arg::Id(value) => buf.extend(value.to_ne_bytes()),
        }
    }
}

struct Id(u64);

struct NewId(u64);

trait OwnedArg: Sized {
    // TODO fds?
    fn parse(buf: &mut ByteStream) -> Option<Self>;
}

impl OwnedArg for u32 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for i32 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for u64 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for i64 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for f32 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

// XXX how are fds grouped in stream?
impl OwnedArg for OwnedFd {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        // XXX error?
        buf.read_fd()
    }
}

impl OwnedArg for String {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        let mut len = u32::parse(buf)?;
        let bytes = buf.read_n(len as usize - 1)?; // Exclude NUL
                                                   // XXX error?
        let string = String::from_utf8(bytes.to_owned()).ok()?;
        // Padding
        while len % 4 != 0 {
            len += 1;
            buf.read::<1>()?;
        }
        Some(string)
    }
}

impl OwnedArg for NewId {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        u64::parse(buf).map(Self)
    }
}

impl OwnedArg for Id {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        u64::parse(buf).map(Self)
    }
}

trait Interface {
    const NAME: &'static str;
    const VERSION: u32;
    type Incoming;
}

// XXX pub
pub struct ByteStream<'a> {
    pub connection: &'a Connection,
    pub bytes: &'a [u8],
    pub fds: &'a mut Vec<OwnedFd>,
}

impl<'a> ByteStream<'a> {
    fn connection(&self) -> &Connection {
        &self.connection
    }

    fn read_n(&mut self, n: usize) -> Option<&[u8]> {
        if self.bytes.len() >= n {
            let value;
            (value, self.bytes) = self.bytes.split_at(n);
            Some(value)
        } else {
            None
        }
    }

    fn read<const N: usize>(&mut self) -> Option<[u8; N]> {
        if self.bytes.len() >= N {
            let value;
            (value, self.bytes) = self.bytes.split_at(N);
            Some(value.try_into().unwrap())
        } else {
            None
        }
    }

    fn read_fd(&mut self) -> Option<OwnedFd> {
        if !self.fds.is_empty() {
            Some(self.fds.remove(0))
        } else {
            None
        }
    }

    fn read_arg<T: OwnedArg>(&mut self) -> Option<T> {
        T::parse(self)
    }
}

#[derive(Debug)]
struct ObjectInner {
    connection: Connection,
    id: u64,
    destroyed: AtomicBool,
}

#[derive(Clone, Debug)]
struct Object(Arc<ObjectInner>);

impl Object {
    fn new(connection: Connection, id: u64) -> Self {
        Self(Arc::new(ObjectInner {
            connection,
            id,
            destroyed: AtomicBool::new(false),
        }))
    }

    fn connection(&self) -> &Connection {
        &self.0.connection
    }

    fn id(&self) -> u64 {
        self.0.id
    }

    fn request(&self, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
        self.connection().request(self.id(), opcode, args)
    }
}

#[cfg(test)]
mod tests {
    // TODO add serialization/deserialization tests
}
