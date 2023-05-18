use rustix::{
    io::{IoSlice, IoSliceMut},
    net,
};
use std::{
    env,
    os::unix::{
        io::{BorrowedFd, OwnedFd},
        net::UnixStream,
    },
    path::PathBuf,
};

#[allow(unused_parens)]
pub mod ei {
    include!("eiproto_ei.rs");
}

#[allow(unused_parens)]
pub mod eis {
    include!("eiproto_eis.rs");
}

struct Connection {
    socket: UnixStream,
}

impl Connection {
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
}

// Want to fallback to higher number if exists, on server?
// create on server, not client.
fn socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}
