#![forbid(unsafe_code)]

use rustix::{
    io::{IoSlice, IoSliceMut},
    net,
};
use std::{
    env,
    os::unix::{
        io::{BorrowedFd, OwnedFd},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::Arc,
};

#[allow(unused_parens)]
pub mod ei {
    include!("eiproto_ei.rs");
}

#[allow(unused_parens)]
pub mod eis {
    pub struct Listener {
        listener: super::UnixListener
    }

    impl Listener {
        pub fn bind(path: &super::Path) -> std::io::Result<Self> {
            Ok(Self {
                listener: super::UnixListener::bind(path)?
            })
        }

        pub fn incoming(&self) -> impl Iterator<Item = super::Connection> + '_ {
            self.listener.incoming().filter_map(Result::ok).map(|socket| {
                super::Connection {
                    socket: super::Arc::new(socket)
                }
            })
        }
    }

    include!("eiproto_eis.rs");
}

// TODO Listener?
// TODO versioning?

#[derive(Clone, Debug)]
struct Connection {
    socket: Arc<UnixStream>,
}

impl Connection {
    // TODO EINTR
    // TODO send return value? send more?
    fn send(&self, data: &[u8], fds: &[BorrowedFd]) -> rustix::io::Result<()> {
        let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(fds.len()))];
        let mut cmsg_buffer = net::SendAncillaryBuffer::new(&mut cmsg_space);
        cmsg_buffer.push(net::SendAncillaryMessage::ScmRights(&fds));
        net::sendmsg_noaddr(
            &self.socket,
            &[IoSlice::new(data)],
            &mut cmsg_buffer,
            net::SendFlags::empty(),
        )?;
        Ok(())
    }

    fn recv(&self, buf: &mut [u8], fds: &mut Vec<OwnedFd>) -> rustix::io::Result<usize> {
        const MAX_FDS: usize = 32;

        let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(MAX_FDS))];
        let mut cmsg_buffer = net::RecvAncillaryBuffer::new(&mut cmsg_space);
        let response = net::recvmsg(
            &self.socket,
            &mut [IoSliceMut::new(buf)],
            &mut cmsg_buffer,
            net::RecvFlags::empty(),
        )?;
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

    fn new_id(&self) -> u64 {
        // TODO
        42
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

// Want to fallback to higher number if exists, on server?
// create on server, not client.
fn socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}

#[derive(Debug)]
struct Header {
    object_id: u64,
    length: u32,
    opcode: u32,
}

impl Header {
    fn parse(bytes: &[u8]) -> Option<Self> {
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
                buf.extend((0..(len - len % 4)).map(|_| b'\0'));
            }
            Arg::NewId(value) => buf.extend(value.to_ne_bytes()),
            Arg::Id(value) => buf.extend(value.to_ne_bytes()),
        }
    }
}

#[derive(Debug)]
enum OwnedArg {
    Uint32(u32),
    Int32(i32),
    Uint64(u64),
    Int64(i64),
    Float(f32),
    Fd(OwnedFd),
    String(String),
    NewId(u64),
    Id(u64),
}
