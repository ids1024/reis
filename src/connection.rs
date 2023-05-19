use rustix::{
    io::{retry_on_intr, IoSlice, IoSliceMut},
    net,
};
use std::{
    collections::HashMap,
    io,
    os::unix::{
        io::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
        net::UnixStream,
    },
    sync::{Arc, Mutex},
};

use crate::{Arg, Header, Object};

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
    pub(crate) fn new(socket: UnixStream, client: bool) -> io::Result<Self> {
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
    pub fn eis_handshake(&self) -> crate::eis::handshake::Handshake {
        crate::eis::handshake::Handshake(Object::new(self.clone(), 0))
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
